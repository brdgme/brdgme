# web-domain review LOG

Lead session started 2026-07-24. Snapshot: `/home/beefsack/Development/brdgme-review-snapshot` @ `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`.

## Scope (~14.2k LOC, fits one unit)

| File | LOC |
|---|---|
| src/game/server_fns.rs | 2479 |
| src/game/mod.rs | 1101 |
| src/game/import.rs | 369 |
| src/game/export.rs | 223 |
| src/proposals.rs | 2961 |
| src/stats/queries.rs | 2076 |
| src/stats/mod.rs | 353 |
| src/stats/viz.rs | 326 |
| src/players.rs | 1189 |
| src/friends.rs | 581 |
| src/new_game.rs | 660 |
| src/rules.rs | 548 |
| src/settings.rs | 572 |
| src/game_info/queries.rs | 337 |
| src/game_info/mod.rs | 203 |
| src/models/game.rs | 92 |
| src/models/user.rs | 38 |
| src/models/mod.rs | 5 |
| src/index.rs | 107 |

## Handoffs from web-server unit (to resolve)

1. `findings/web-server.md` nats finding "Messages that exhaust `max_deliver=3` strand silently": does `web::game::run_bot_command_consumer` `term()` poison messages? term() → downgrade to nit; no term() → upgrade to major.
2. `ack_wait = 5 min` vs consumer processing time: confirm consumer acks promptly or sends `in_progress()` pings; otherwise raise ack_wait. Check consumer ack cadence in src/game/.

## Worker plan (serial)

- W1: game/mod.rs + game/export.rs + game/import.rs + NATS handoff resolution → raw `web-domain-game-mod.md`
- W2: game/server_fns.rs → raw `web-domain-game-serverfns.md`
- W3: proposals.rs → raw `web-domain-proposals.md`
- W4: stats/ → raw `web-domain-stats.md`
- W5: players.rs + friends.rs + new_game.rs → raw `web-domain-social.md`
- W6: game_info/ + models/ + rules.rs + settings.rs + index.rs → raw `web-domain-misc.md`

## Dispatch / return log

### W1 dispatched → returned (game/mod.rs, export.rs, import.rs + NATS handoff)
- Raw file: `findings/raw/web-domain-game-mod.md` — 13 findings (0 crit / 4 major / 4 minor / 5 nit; correctness 9, quality 3, consistency 2).
- Headlines: (1) bot-turn wedge in 3 loss modes — UserError acked w/o re-publish (mod.rs:304-314), conflict-retry exhaustion acked (mod.rs:372-383), bot.turn publish failure warn-only post-commit (mod.rs:227-242); game stuck forever. (2) consumer spawned once, never restarted (main.rs:55-74). (3) no term()/DLQ → max_deliver=3 messages strand silently. (4) finished games wipe is_eliminated (UNCERTAIN, minor). (5) export bundle includes private log bodies (minor, decision needed).
- NATS HANDOFF RESOLVED: no `.term(`/`.nak(`/`.in_progress` anywhere in web/src (grep = 0 matches). Ack exactly once after ALL work (mod.rs:300/311); no in_progress pings; worst-case processing tens of seconds (10s timeout × 3 attempts) < 5-min ack_wait → duplicate delivery unlikely but unguarded. Unparseable payload → ack (poison drop, deliberate); UserError → ack w/o re-publish (wedge); Other → unacked → 3 deliveries → strand. Ack failure warn-only (mod.rs:301). No dedup on `attempt`; idempotency via fresh state read (mod.rs:89) + updated_at CAS (db.rs:1715-1728). → web-server nats finding "stranded messages" CONFIRMED (stays minor; silent, operationally invisible); ack_wait finding mostly closed (processing bounded well under 5 min, but no guard).
- Lead verification: DONE (2026-07-24, new session). Spot-checked against snapshot source:
  - VERIFIED UserError ack w/o bot.turn re-publish (mod.rs:304-314 ack; handler 402-410 returns UserError, no re-publish) - wedge confirmed, major stands.
  - VERIFIED conflict exhaustion returns Ok(()) at attempt >= MAX_TURN_ATTEMPTS (mod.rs:372-383) - acked away, major stands.
  - VERIFIED publish_bot_turns failure paths warn-only, both send Err and persistence-ack Err (mod.rs:227-242) - major stands.
  - VERIFIED consumer spawn-and-forget, no restart loop (main.rs:55-74; run_bot_command_consumer returns Ok(()) on stream end mod.rs:322-325) - major stands.
  - VERIFIED Status::Finished arm emits eliminated: vec![] (mod.rs:36-41) - minor stands.
  - VERIFIED export includes private log bodies + target positions (export.rs:105-134) - minor stands.
  - No findings rejected or downgraded. All 13 accepted as-is.

### W2 dispatched -> returned (game/server_fns.rs)
- Raw file: `findings/raw/web-domain-game-serverfns.md` - 12 findings (1 crit / 3 major / 4 minor / 4 nit).
- Headlines: (crit) undo_game allows undoing a finished game - ratings never rewound + apply_rating_changes idempotency guard (db.rs:1554) then blocks rating the real outcome; (major) undo has no updated_at CAS (db.rs:1416-1417 unconditional UPDATE) so it clobbers concurrent moves; concede_game TOCTOU (no FOR UPDATE / NOT is_finished guard); get_game_details has no visibility gate - db::is_game_visible_to_user is dead code, any logged-in user spectates private games.
- Lead verification: DONE. Spot-checked source:
  - VERIFIED undo_game (server_fns.rs:731-804) has no is_finished check; db::undo_game (db.rs:1407-1449) has no updated_at guard and never touches rating_change; idempotency guard confirmed at db.rs:1554-1557. Critical stands.
  - VERIFIED get_game_details (server_fns.rs:231-260) renders for any authenticated user, no visibility check. Major stands.
  - No findings rejected or downgraded. All 12 accepted.

### W3 dispatched -> returned (proposals.rs)
- Raw file: `findings/raw/web-domain-proposals.md` - 18 findings (0 crit / 3 major / 10 minor / 5 nit).
- Headlines: (major) get_proposal ships every invitee's email_token to any authenticated viewer (ViewerRole computed but never gates data); client-supplied BotSlot unvalidated in create_proposal + add_proposal_player (same class as W2 restart_core, feeds W1 wedge); auto-decline sweep keys on proposal created_at not player row age - late invitees / roster-reset players terminally auto-declined.
- Lead verification: DONE. Spot-checked source:
  - VERIFIED ProposalPlayerView.email_token (proposals.rs:78) returned via get_proposal (1744-1764); viewer_role computed but players list unfiltered. Major stands.
  - VERIFIED fetch_auto_decline_candidates keys on gp.created_at (game_proposals alias, proposals.rs:812-819). Major stands.
  - BotSlot validation gap accepted on the strength of W2's verified parallel in restart_core.
  - No findings rejected or downgraded. All 18 accepted.

### W4 dispatched -> returned (stats/)
- Raw file: `findings/raw/web-domain-stats.md` - 11 findings (0 crit / 1 major / 7 minor / 3 nit).
- Headlines: (major) all three stats server fns anonymous-accessible with no game_visibility check - opponent identities/placements/game ids leak past friends/private settings (same class as W2's get_game_details gate, separate endpoints). Minors: page offset overflow (mod.rs:318), hardcoded 1200 base rating, unbounded game-type payloads, 7x duplicated single-human predicate, 4 correlated subqueries per history row, exact-match game_type filter.
- Lead verification: DONE. Spot-checked source:
  - VERIFIED get_player_profile (stats/mod.rs:174-222) takes viewer as Option, no visibility gate anywhere before returning games/opponents. Major stands.
  - VERIFIED `(page - 1) * page_size` unclamped i64 at mod.rs:316-318. Minor stands.
  - No findings rejected or downgraded. All 11 accepted.

### W5 dispatched -> returned (players.rs, friends.rs, new_game.rs)
- Raw file: `findings/raw/web-domain-social.md` - 12 findings (0 crit / 0 major / 5 minor / 7 nit).
- Headlines: send_friend_request SELECT-then-INSERT race -> unique-violation 500 instead of mutual-intent auto-accept; friends page swallows errors from 5 of 6 mutation actions; restart prefill Err silently discarded; email slots untrimmed (client end of W3's canonicalization finding).
- Lead verification: DONE. Spot-checked source:
  - VERIFIED send_friend_request has no row lock / ON CONFLICT (db.rs:1877-1904, plain SELECT then INSERT). Minor stands.
  - Authz-clean claims consistent with W2's independent read of get_restart_prefill_impl. No findings rejected or downgraded. All 12 accepted.

### W6 dispatched -> returned (game_info/, models/, rules.rs, settings.rs, index.rs)
- Raw file: `findings/raw/web-domain-misc.md` - 14 findings (0 crit / 1 major / 7 minor / 6 nit).
- Headlines: (major) game_info rules version picked by `ORDER BY name LIMIT 1` - links oldest version's rules, lexicographic semver break. Minors: get_rendered_rules auth-gated from anonymous page + ignores is_public; unterminated fence silently dropped; settings add-email untrimmed (W3 parallel); fire-and-forget settings mutations; index O(friends x 10) sequential queries.
- Lead verification: DONE. Spot-checked source:
  - VERIFIED `ORDER BY name LIMIT 1` at game_info/queries.rs:14-19. Major stands.
  - No findings rejected or downgraded. All 14 accepted.

## Curation complete (2026-07-24)

- Curated file: `findings/web-domain.md`. Unit 10 web-domain COMPLETE.
- Raw totals across W1-W6: 80 findings (1 crit / 12 major / 36 minor / 31 nit). Note: the original W1 return entry above miscounted W1 as 4m/5n; actual raw blocks are 4M/3m/6n.
- MERGED: W2 "restart_core accepts arbitrary bot_name" (minor) + W3 "Client-supplied BotSlot unvalidated" (major) -> single major covering create_proposal/add_proposal_player/restart_core (identical unrecoverable-wedge consequence; effectively upgraded the restart_core instance).
- MERGED: W4 "page offset overflow" (minor, stats/mod.rs:318) + W5 "unbounded page number forwarded" (nit, players.rs:771) -> single minor with both locations.
- No findings rejected. No severity downgrades.
- Curated tally: 1 critical / 12 major / 35 minor / 30 nit = 78 findings.

## Handover (2026-07-24)

- Review handed over from prior Kimi K3 session to a new Claude Code session. State surveyed; context doc written to `docs/reviews/HANDOVER.md`. Resume at W1 verification, then dispatch W2-W6 per the plan above. No review content changed during handover.
