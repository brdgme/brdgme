# W4 triage: games-batch-f + web-server

## games-batch-f: 58 findings (0c / 2M / 22m / 34n) - all survive verification, tallies match expected
## web-server: 66 surviving findings (0c / 8M / 36m / 22n) - F30 REJECTED (67 raw); adjusted severities applied (F1 c->M, F18 M->m, F27 m->M, F58 m->n); tallies match expected

games-batch-f F1 | major | zombie-dice pub_state exposes shuffled cup in draw order - next draws readable by all | game/zombie-dice-2/src/lib.rs, game/zombie-dice-2/DATA_DOCS.md | D | pubstate-leaks (pub_state redaction shape)
games-batch-f F2 | minor | Cup refill returns shotgun dice to cup, deviating from official SJG rules (Go quirk) | game/zombie-dice-2/src/lib.rs, game/zombie-dice-2/RULES.md | D | port-parity
games-batch-f F3 | minor | Rolloff state (roll_off_players) never exposed in PubState or render | game/zombie-dice-2/src/lib.rs, game/zombie-dice-2/src/render.rs | M | zombie-dice
games-batch-f F4 | minor | Panic paths (drain, scores index, % players) on inconsistent deserialized state | game/zombie-dice-2/src/lib.rs, game/zombie-dice-2/src/render.rs | M | state-trust-panics
games-batch-f F5 | nit | Unbounded recursion roll->next_player->start_turn on repeated busts (theoretical) | game/zombie-dice-2/src/lib.rs | M | zombie-dice
games-batch-f F6 | nit | Rolloff tie announcement re-logged on every wrap while still tied | game/zombie-dice-2/src/lib.rs | M | zombie-dice
games-batch-f F7 | nit | Duplicated ~15-line finish-handling block in both command() arms | game/zombie-dice-2/src/lib.rs | M | zombie-dice
games-batch-f F8 | minor | shoot() drops Go's bounds check; direct indexing can panic on out-of-range Loc | game/battleship-2/src/lib.rs | M | battleship
games-batch-f F9 | minor | Indexing trusts players/Vec lengths; inconsistent with defensive .get() elsewhere | game/battleship-2/src/lib.rs | M | state-trust-panics
games-batch-f F10 | nit | expect("cell is a ship") in shoot sunk-detection branch | game/battleship-2/src/lib.rs | M | battleship
games-batch-f F11 | nit | Ship::all() vs Direction::all() inconsistent return types | game/battleship-2/src/lib.rs, game/battleship-2/src/command.rs | M | battleship
games-batch-f F12 | nit | Hit-count helpers return i32 for non-negative counts | game/battleship-2/src/lib.rs | M | battleship
games-batch-f F13 | major | Selling-phase secret plays leaked to everyone via PubState.bids | game/for-sale-2/src/lib.rs | D | pubstate-leaks (pub_state redaction shape)
games-batch-f F14 | minor | Passing pays floor(bid/2); official rules round up (Go quirk) | game/for-sale-2/src/lib.rs, game/for-sale-2/RULES.md | D | port-parity
games-batch-f F15 | minor | Deck/chip setup deviates from official For Sale (Go quirk) | game/for-sale-2/src/lib.rs | D | port-parity
games-batch-f F16 | minor | RULES.md cheque deck description factually wrong; tie sentence contradicts code | game/for-sale-2/RULES.md | M | for-sale
games-batch-f F17 | minor | End-of-game "scores" log shows cheque totals only, omitting chips | game/for-sale-2/src/lib.rs | M | for-sale
games-batch-f F18 | minor | Phase inferred from deck sizes via SELL_THRESHOLD magic; no stored Phase | game/for-sale-2/src/lib.rs | M | for-sale (use Option<Phase> migration, NOT serde(default))
games-batch-f F19 | nit | Panic-on-empty-deck paths reachable only from corrupt state | game/for-sale-2/src/lib.rs | M | state-trust-panics
games-batch-f F20 | nit | Selling autoplay keys off only player 0's hand size | game/for-sale-2/src/lib.rs | M | for-sale
games-batch-f F21 | nit | Tie ranking diverges from Go GenPlacings (dense vs standard competition) - lib-level | game/for-sale-2/src/lib.rs, lib/game/src/game.rs | D | port-parity (lib gen_placings ranking)
games-batch-f F22 | nit | render::highest_bid duplicates game logic with different sentinel | game/for-sale-2/src/render.rs, game/for-sale-2/src/lib.rs | M | for-sale
games-batch-f F23 | nit | Helper methods unnecessarily pub; player_state indexes unchecked | game/for-sale-2/src/lib.rs | M | for-sale
games-batch-f F24 | minor | Player cap 8 contradicts Go/official 2-10 and crate's own RULES.md | game/category-5-2/src/lib.rs, game/category-5-2/RULES.md | M | category-5
games-batch-f F25 | minor | draw_cards recurses without bound - stack overflow if n exceeds deck+discard | game/category-5-2/src/lib.rs | M | category-5
games-batch-f F26 | nit | Panic-on-invariant expect calls in resolve/choose paths | game/category-5-2/src/lib.rs | M | state-trust-panics
games-batch-f F27 | nit | "N points until end of game" renders negative after game ends | game/category-5-2/src/render.rs | M | category-5
games-batch-f F28 | nit | points() returns lower-is-better totals; label display only, do NOT negate (ELO uses place) | game/category-5-2/src/lib.rs | M | category-5
games-batch-f F29 | nit | Card(pub u8) permits invalid cards via public field and serde | game/category-5-2/src/lib.rs | M | state-trust-panics
games-batch-f F30 | nit | Test comment typo: "11 is a multiple of 1 only" | game/category-5-2/src/lib.rs | M | category-5
games-batch-f F31 | nit | hands[0] used as proxy for all hand sizes in resolve_plays; invariant implicit | game/category-5-2/src/lib.rs | M | category-5
games-batch-f F32 | minor | Game::score ignores player arg and skips turn validation | game/greed-2/src/lib.rs | M | greed
games-batch-f F33 | minor | E/e score-token collision: "score eee" consumes E1 triple first (Go quirk) | game/greed-2/src/command.rs, game/greed-2/src/lib.rs | D | port-parity
games-batch-f F34 | minor | Scores/dice length invariants unchecked on deserialized state | game/greed-2/src/lib.rs, game/greed-2/src/render.rs | M | state-trust-panics
games-batch-f F35 | nit | Duplicated placings-log block in Roll and Done command arms | game/greed-2/src/lib.rs | M | greed
games-batch-f F36 | nit | Theoretical i32 overflow in turn/banked score arithmetic | game/greed-2/src/lib.rs | M | greed
games-batch-f F37 | nit | Die::E1 rendered Foreground though RULES.md says black | game/greed-2/src/lib.rs, game/greed-2/RULES.md | M | greed
games-batch-f F38 | minor | Scoring table duplicated between lib.rs SCORES and render.rs help table | game/farkle-2/src/render.rs, game/farkle-2/src/lib.rs | M | farkle
games-batch-f F39 | nit | score() pub but ignores player arg; siblings validate | game/farkle-2/src/lib.rs | M | farkle
games-batch-f F40 | nit | Finished game shows stale turn_score/remaining_dice in pub_state/render | game/farkle-2/src/lib.rs | M | farkle
games-batch-f F41 | nit | Test sets out-of-range current_player (first_player + 1 unclamped) | game/farkle-2/src/lib.rs | M | farkle
games-batch-f F42 | nit | render.rs uses u8 instead of the Die alias | game/farkle-2/src/render.rs | M | farkle
games-batch-f F43 | nit | Simplified Farkle variant (no straight/pairs, 5000 target) - Go-faithful cross-ref | game/farkle-2/src/lib.rs, game/farkle-2/RULES.md | D | port-parity
games-batch-f F44 | nit | PubState renderer indexes scores[p] without length check | game/farkle-2/src/render.rs | M | state-trust-panics
games-batch-f F45 | minor | `1 - start_player` usize underflow on crafted state (wraps in release) | game/tic-tac-toe-2/src/render.rs | M | tic-tac-toe
games-batch-f F46 | minor | Crafted players count drives unbounded allocation/iteration (systemic requester trust) | game/tic-tac-toe-2/src/lib.rs, lib/cmd/src/requester/gamer.rs | M | state-trust-panics
games-batch-f F47 | nit | Dead, misleading Cell::Empty arm in winner() | game/tic-tac-toe-2/src/lib.rs | M | tic-tac-toe
games-batch-f F48 | nit | Mark casing inconsistent (log+label uppercase vs board+RULES.md lowercase) | game/tic-tac-toe-2/src/lib.rs, game/tic-tac-toe-2/src/render.rs | M | tic-tac-toe (fix must touch exact-render test)
games-batch-f F49 | minor | Vacuous test_init_player_chips asserts nothing (loop over 0 players) | game/no-thanks-2/src/lib.rs | M | no-thanks
games-batch-f F50 | minor | Player cap 3-5 vs later-edition official 3-7 (Go quirk; 2004 edition was 3-5) | game/no-thanks-2/src/lib.rs | D | port-parity
games-batch-f F51 | nit | Unreachable "no chips" branch in pass(); fold message rather than delete | game/no-thanks-2/src/lib.rs | M | no-thanks
games-batch-f F52 | nit | Run-grouping logic duplicated between lib.rs and render.rs | game/no-thanks-2/src/lib.rs, game/no-thanks-2/src/render.rs | M | no-thanks
games-batch-f F53 | nit | Renderer panics on inconsistent deserialized PubState (also chips/final_scores) | game/no-thanks-2/src/render.rs, game/no-thanks-2/src/lib.rs | M | state-trust-panics
games-batch-f F54 | minor | Turn after challenge goes past caller, not to challenge loser (Go quirk) | game/liars-dice-2/src/lib.rs, game/liars-dice-2/RULES.md | D | port-parity
games-batch-f F55 | minor | Index panics reachable from inconsistent deserialized state | game/liars-dice-2/src/lib.rs | M | state-trust-panics
games-batch-f F56 | nit | "fourty" typo from Go; reachable via ordinary input (uncapped bids, see F57) | game/liars-dice-2/src/render.rs | M | liars-dice
games-batch-f F57 | nit | Bid quantity has no upper bound in the parser | game/liars-dice-2/src/command.rs | M | liars-dice
games-batch-f F58 | nit | Test gaps: hidden-info redaction, wild-1 call resolution, full game | game/liars-dice-2/src/lib.rs, game/liars-dice-2/tests/contract.rs | M | liars-dice
web-server F1 | major | Concurrent confirm requests bypass per-code attempt cap (check-then-act, no lock) | web/src/auth/server.rs | M | auth-races (atomic UPDATE; compare with > not >=)
web-server F2 | major | Email-squatting: unverified add_email_address blocks real owner's signup forever | web/src/auth/server.rs, web/src/db.rs | D | auth-edges (auth-edge semantics: code as proof of ownership)
web-server F3 | minor | No session ID rotation (cycle_id) on login | web/src/auth/server.rs | M | auth-session
web-server F4 | minor | Blocked-domain check leaks account existence via differential response | web/src/auth/server.rs | D | auth-edges (auth-edge semantics; uniform-reject alt locks out verified users)
web-server F5 | minor | Transient DB error in get_current_user clears the session (mass logout) | web/src/auth/server.rs | M | auth-session
web-server F6 | minor | Global 24h send cap over-counts historical sends (cumulative sent_count) | web/src/auth/server.rs | M | send-caps (only windowed-counter fix is sound; reset-on-rotation under-counts)
web-server F7 | minor | Resend failure silent, consumes quota, can lock user out for the window | web/src/auth/server.rs | D | send-caps (fail-open policy: surface vs not-count failed sends)
web-server F8 | minor | Turnstile fails open on verifier errors; unset secret silently disables it | web/src/auth/server.rs | D | fail-open (fail-open policy)
web-server F9 | minor | Emails not normalized (case/whitespace) anywhere in auth flow | web/src/auth/server.rs | D | auth-edges (normalization policy; existing rows affected)
web-server F10 | minor | add_email_address discards send-cap refusal and reports success | web/src/auth/server.rs | M | send-caps
web-server F11 | minor | Auth tokens never expire DB-side, never GC'd, no revoke-all path | web/src/auth/session.rs, web/src/db.rs | D | auth-session (confirm intentional design vs add expiry/GC)
web-server F12 | nit | 6-digit code: modulo bias, non-CT compare, plaintext storage | web/src/auth/server.rs | M | auth-polish
web-server F13 | nit | confirm_login does not validate email/token shape | web/src/auth/server.rs | M | auth-polish
web-server F14 | nit | New reqwest::Client built per Turnstile verification | web/src/auth/server.rs | M | auth-polish (expect_context in server-fn scope, pass client in)
web-server F15 | nit | Logout swallows token-invalidation error; session record not flushed | web/src/auth/server.rs, web/src/auth/session.rs | M | auth-polish
web-server F16 | major | Hardcoded public fallback key silently used when DATABASE_ENCRYPTION_KEY unset | web/src/crypto.rs, web/src/main.rs | D | fail-open (fail-open policy: refuse startup outside explicit dev mode)
web-server F17 | nit | No AAD context binding and no key zeroization | web/src/crypto.rs | M | crypto-hardening
web-server F18 | minor | reorder_bots is not atomic (N updates, no tx, dup display_order possible) | web/src/admin.rs | M | admin-mutations
web-server F19 | minor | create_bot display_order MAX+1 race | web/src/admin.rs | M | admin-mutations
web-server F20 | minor | API-key mask exposes whole key for keys <=4 chars; fabricated sk- prefix | web/src/admin.rs | M | admin-polish
web-server F21 | minor | No way to clear a provider API key once set (None vs unset conflated) | web/src/admin.rs | D | admin-mutations (key-clearing UX wanted or document)
web-server F22 | minor | test_provider hardcodes model "gpt-4o-mini" - false negatives for other providers | web/src/admin.rs | M | admin-polish
web-server F23 | minor | Upstream test response body/headers returned to client uncapped | web/src/admin.rs | M | admin-polish
web-server F24 | minor | Test result can attribute to wrong row; key off value() alone (dispatched counter also never incremented) | web/src/admin.rs | M | admin-actions
web-server F25 | minor | No server-side validation of bot/provider inputs (all client-side) | web/src/admin.rs | M | admin-polish
web-server F26 | minor | Admin page hard-fails entirely if one provider key is undecryptable | web/src/admin.rs | M | admin-polish
web-server F27 | major | Bot rename/delete/disable permanently deadlocks in-flight games (worker acks + skips) | web/src/admin.rs, bot/src/main.rs | D | bot-pipeline (bot-by-name migration)
web-server F28 | minor | Admin-gate boilerplate duplicated 15 times | web/src/admin.rs | M | admin-gate-dup
web-server F29 | minor | Mutations don't verify rows_affected; no-op updates report success | web/src/admin.rs | M | admin-mutations
web-server F31 | nit | AdminPage non-admin redirect matches on error-message text | web/src/admin.rs | M | admin-polish
web-server F32 | nit | Action-value double-get .unwrap()s in completion Effects (10x) | web/src/admin.rs | M | admin-actions
web-server F33 | nit | Local type BotProviderRow shadows the public struct | web/src/admin.rs | M | admin-polish
web-server F34 | major | undo_game keeps voided ratings; re-finished game never re-rated (no is_finished guard on undo path) | web/src/db.rs, web/src/game/server_fns.rs | D | ratings (rating-rewind policy; recompute-only double-counts)
web-server F35 | major | ~20 public DB functions untested (drop choose_colors/ELO from list - those ARE tested) | web/src/db.rs | M | db-tests
web-server F36 | minor | Redundant updated_at=NOW() on trigger-maintained tables (EXCLUDE :1357/:1363 game_proposals - required there) | web/src/db.rs | M | db-updated-at
web-server F37 | minor | update_game_command_success can leave is_finished=false with finished_at set | web/src/db.rs | M | db-lifecycle
web-server F38 | minor | Zero-change rating results leave rating_change NULL, defeating idempotency guard | web/src/db.rs | M | ratings
web-server F39 | minor | send_friend_request opposite-direction race hits raw 23505 error | web/src/db.rs | M | db-races
web-server F40 | minor | friend_recent_visible_game is N+1 by construction | web/src/db.rs | M | db-perf (keep cross-ref to centralized visibility predicate if inlined)
web-server F41 | minor | insert_game_logs_tx is row-at-a-time inside the transaction | web/src/db.rs | M | db-perf
web-server F42 | minor | db.rs is a 6.4k-line grab-bag (well-sectioned); split when next touched | web/src/db.rs | M | db-structure
web-server F43 | minor | build_game_type_user silently fabricates default rating row (nil-id sentinel undocumented) | web/src/db.rs | M | db-structure
web-server F44 | nit | is_turn_at reset for continuing-turn players; fights the trigger | web/src/db.rs | D | db-lifecycle (is_turn_at semantics: turn-started vs last-activity)
web-server F45 | nit | is_user_admin returns sqlx::Result while neighbors use anyhow::Result | web/src/db.rs | M | db-structure
web-server F46 | nit | generate_unique_username check-then-act race (mitigated by unique index) | web/src/db.rs | M | db-races
web-server F47 | nit | Interval built via string interpolation of bound param; use make_interval | web/src/db.rs | M | db-polish
web-server F48 | nit | No app-level self-friend-request guard (DB CHECK only) | web/src/db.rs | M | db-polish (silent-Ok fix breaks existing Err assertion at :3317 - change test with it)
web-server F49 | nit | choose_colors clones the whole prefs vec each outer-loop pass | web/src/db.rs | M | db-polish
web-server F50 | nit | apply_rating_changes convoluted all-pairs loop idiom | web/src/db.rs | M | db-polish
web-server F51 | nit | Test-quality nits: 3-player visibility gap, format!-built count_rows (part 1 rejected) | web/src/db.rs | M | db-tests
web-server F52 | major | Session cookies lack Secure flag in prod (SECURE_COOKIE never set anywhere) | web/src/auth/session.rs, k8s/base/web/deployment.yaml, web/.env.template | M | deploy-hardening (default-secure-in-code; base env var would force dev overlay)
web-server F53 | major | Bot command consumer unsupervised - silent permanent bot outage on exit/panic | web/src/main.rs, web/src/game/mod.rs | M | bot-pipeline
web-server F54 | minor | No rustls CryptoProvider installed in web's main (dual-provider panic risk) | web/src/main.rs | M | deploy-hardening
web-server F55 | minor | Graceful shutdown does not cover WS connections or background tasks | web/src/main.rs, web/src/websocket.rs | M | deploy-hardening
web-server F56 | minor | Messages exhausting max_deliver=3 strand silently (transient-failure class only; no DLQ/advisory) | web/src/nats.rs, web/src/game/mod.rs | M | bot-pipeline
web-server F57 | minor | get_or_create_stream/consumer never reconcile config drift | web/src/nats.rs | M | nats-config
web-server F58 | nit | ack_wait=5min vs processing time - theoretical only (processing bounded well under) | web/src/nats.rs | M | nats-config
web-server F59 | minor | /ws has no authentication; every connection gets site-wide firehose | web/src/websocket.rs, web/src/router.rs | D | ws-firehose (/ws firehose: accept+document vs per-connection subscribe)
web-server F60 | minor | Client open() on every visibilitychange/online tears down healthy sockets | web/src/websocket_client.rs | M | ws-client (gate on ready_state.get_untracked() in listeners)
web-server F61 | nit | WS inbound message/frame limits left at tungstenite defaults (~64MiB) | web/src/websocket.rs | M | ws-hardening
web-server F62 | nit | No dead-connection detection beyond send failure (pongs never checked) | web/src/websocket.rs | M | ws-hardening
web-server F63 | nit | Unbounded file read in import_game (dev-only tool) | web/src/bin/import_game.rs | M | misc
web-server F64 | minor | gloo-net dependency is unused (lands in WASM bundle) | web/Cargo.toml | M | cargo-deps
web-server F65 | minor | tokio net and time features used but not declared | web/Cargo.toml | M | cargo-deps
web-server F66 | nit | futures-util non-optional but only used in ssr/test code | web/Cargo.toml | M | cargo-deps (naive optional breaks non-ssr cargo test; needs dev-dep or required-features)
web-server F67 | nit | Dependency currency: async-nats 0.50 and svix 1.99.1 available | web/Cargo.toml | M | cargo-deps

## Grouping notes

Natural package boundaries:
- pubstate-leaks (games F1, F13): the unit's only two majors, same mechanism (pub_state serializes hidden info verbatim). Needs one design decision on redaction shape (redact vs canonicalize vs per-player re-add in player_state) then two small mechanical fixes. games F3 (rolloff not in PubState) is adjacent zombie-dice pub_state work and could ride along.
- port-parity (games F2, F14, F15, F21, F33, F43, F50, F54): all cross-references for the project-wide "Go parity vs official rules" ruling (joins modern-art-2 payout, splendor-2 tie-break etc.). Zero code until the ruling; then per-crate one-liners plus RULES.md updates. F21 is really a lib/game gen_placings decision, not per-crate.
- state-trust-panics (games F4, F9, F19, F26, F29, F34, F44, F46, F53, F55): systemic "deserialized Game/PubState is trusted" shape across all nine crates. The findings file itself says the fix belongs in the requester/serde layer (lib/cmd/src/requester/gamer.rs), not per-crate. One design decision (validate-on-load hook vs defensive .get() everywhere vs accept), then either one lib-level fix or a mechanical per-crate sweep. Cross-unit: same trust model underlies every game crate reviewed in other batches.
- per-crate slugs (zombie-dice, battleship, for-sale, category-5, greed, farkle, tic-tac-toe, no-thanks, liars-dice): independent small mechanical fixes, each crate a natural half-day package.
- auth work splits into: auth-races (F1 atomic cap), auth-edges (F2/F4/F9 - one semantics session covering squatting, enumeration, normalization), auth-session (F3/F5/F11), send-caps (F6/F7/F10 - same sent_count/quota machinery, fix together), auth-polish nits.
- fail-open (web F8 turnstile, F16 crypto key): same policy question - what fails closed in prod vs degrades in dev. Decide once, apply to both; F16 is the higher-stakes instance.
- admin.rs packages: admin-mutations (F18/F19/F21/F29 - one transaction/rows_affected pass), admin-gate-dup (F28 refactor first makes the rest cleaner), admin-actions (F24/F32 - same Effect pattern), admin-polish grab-bag.
- ratings (web F34, F38): both touch apply_rating_changes idempotency guard; fix together after the rating-rewind policy call. games F28 (points() contract) was discharged by this same machinery - no cross-unit action.
- bot-pipeline (web F27, F53, F56): the end-to-end "bot stops moving silently" theme - name-resolution stranding, unsupervised consumer, poison-message stranding. Strongly coupled operationally; one package with the bot-by-name migration decision (F27) as its design gate.
- deploy-hardening (F52/F54/F55): main.rs/k8s startup-and-shutdown pass.
- cargo-deps (F64-F67): one manifest pass.
- ws: F59 needs the firehose decision; F60-F62 are independent mechanical fixes.

Verification-flagged invalid/unsound recommendations (do NOT apply as originally written):
- games F18: #[serde(default)] phase: Phase migration unsound - Phase defaults to Buying, breaking in-flight Selling games. Use Option<Phase> with current_phase() fallback or post-deserialize fixup.
- games F28: negating points() would be a regression - ELO uses place, never points(); only display/bot-prompt labeling remains.
- games F48: premise partly wrong (label is uppercase); any casing fix must update the exact-render test at lib.rs:589-600.
- games F6: transition-only guard would miss legitimate mid-rolloff membership changes.
- games F11: battleship command.rs:68 needs .to_vec() after the return-type change.
- web F1: increment-before-compare must use > not >= or a correct 10th attempt after 9 failures is rejected.
- web F4: uniform-rejection alternative would lock out existing verified users on blocked domains - inferior option.
- web F6: reset-sent_count-on-rotation UNDER-counts the global cap (hourly rotation = 120 real sends/day vs 50); only the windowed counter is sound.
- web F34: "recompute on next finish" alone double-counts ratings unless game_type_users is also rewound by stored deltas.
- web F36: two listed lines (:1357/:1363) target game_proposals which has NO trigger - a verbatim sweep would break game_proposals.updated_at maintenance.
- web F48: suggested silent-Ok breaks the existing Err assertion at db.rs:3317; change the test deliberately with it.
- web F52: setting SECURE_COOKIE in k8s base/ forces Secure on the dev overlay; prefer default-secure-in-code with explicit opt-out or a prod-only patch.
- web F66: making futures-util optional breaks non-ssr cargo test (ungated uses in tests/); needs a dev-dependency or [[test]] required-features alongside.
- web F30: REJECTED outright - bot_providers has no updated_at column; the recommended fix would be a runtime SQL error. Excluded from rows above.
- web F35: drop choose_colors and ELO helpers from the untested list - both ARE tested (db.rs:5735, :5168).

Cross-unit patterns:
- Hidden-info leaks via pub_state appear in both game units reviewed so far; any redaction-shape decision here should be stated once for all game crates.
- The deserialized-state-trust issue in games-batch-f terminates in web's requester layer (lib/cmd) - a single systemic fix covers ~10 findings in this file alone.
- web F27 + F53 + F56 jointly mean bot outages are currently invisible; whatever supervision/alerting lands for F53 should also cover F56's stranded messages.
