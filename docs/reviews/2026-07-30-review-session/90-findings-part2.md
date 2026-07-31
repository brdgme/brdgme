# Findings part 2 - F-085..F-157 (normalized extraction)

Extraction pass over unit reports 05a, 05b, 06, 07, 07b, 08 plus the out-of-band
`F-96-turnstile-key.md`. No re-review, no new findings.

| ID | Severity | Unit | WP | file:line | Summary | Pairing / status notes |
|----|----------|------|----|-----------|---------|------------------------|
| F-85 | High | 05a | WP-35 | `rust/web/src/auth/server.rs:590-612` (+1 site) | `logout_everywhere` returns `Ok(true)` without deleting any auth-token rows when `get_user_from_session` collapses a session-store error to `None`. | Same root cause as F-86; `logout` (`:566-588`) shares the shape. Systemic pattern 2. Unverifiable by existing tests (harness gap). |
| F-86 | Medium | 05a | WP-34 | `rust/web/src/auth/session.rs:68-74` | `get_user_from_session` swallows session-store errors, so a transient blip silently de-authenticates the user; `get_current_user:555` also discards the `clear_user_session` error. | Read-path half of F-85; ws F5 only half closed by WP-34. |
| F-87 | Medium | 05a | WP-35 | `rust/web/src/auth/server.rs:459-501` | WP-35's F2 fix deletes the pending `user_emails` row of a legitimate in-progress `add_email_address` flow and forks a second account owning the address. | Spec's own case analysis incomplete; needs an owner decision, not a silent behaviour change. |
| F-88 | Low | 05a | WP-34 | `rust/web/src/auth/server.rs:904-924` (+1 site) | `confirm_email_address` passes unvalidated `email`/`token` to `validate_confirmation_code`; WP-34's F13 shape check covers only `confirm_login`. | Feeds F-89 (every unvalidated call burns an attempt). |
| F-89 | Medium | 05a | WP-34 | `rust/web/src/auth/server.rs:394-423` (+1 site) | `validate_confirmation_code` increments `attempts` before any authorization, so any authenticated caller can burn a victim's live login code via `confirm_email_address`. | Direct consequence of WP-34's chosen fix; unconsidered in spec. Amplified by F-94 (no rate limiting). |
| F-90 | Medium | 05a | WP-35 / WP-36 | `rust/bot/src/crypto.rs:59-76` (+1 site) | `rust/bot/src/crypto.rs` is an unhardened duplicate of the web crypto module: hardcoded default key with no `MissingKey`, no `ALLOW_INSECURE_DEFAULT_KEY` gate and no zeroize; its tests pin the old behaviour. | 00-STATE: divergent duplicate, fixes landed only in the web copy. Remediate as ONE item with F-108 (`rust/bot/src/nats.rs` vs `rust/web/src/nats.rs`); Unit 10 owns it. Duplicated-module sweep already done - do not re-run. |
| F-91 | Low | 05a | WP-36 | `rust/web/src/crypto.rs:20-43` (+1 site) | The AAD decline is recorded only in commit `13a1e693`'s message - no `D-NN` entry, no code comment, no spec - and its stated rationale ("shared format with bot") rests on the very duplication F-90 says is unfixed. | Report gives no sub-letters. Coupled to F-90; D-39 ruling applies. |
| F-92 | Medium | 05a | WP-34 | `rust/web/src/auth/server.rs:1001-1996` | Three WP-34-mandated regression tests (F3 session-id rotation, F10 global-cap `Err`, F15 `logout`) were never written, leaving those fixes unverified. | Structural cause is the missing request-parts test harness (see coverage gaps); spec's mandatory-tests criterion unmet. |
| F-93 | Medium | 05a | WP-35 | `rust/web/src/auth/server.rs:853-900` | `add_email_address` returns three distinguishable errors (registered-address oracle) and commits the unverified row before the send, parking rows when the 50/day cap refuses the mail. | D-14 (ii) accepted the asymmetry for `login` only; `add_email_address` was never looked at. |
| F-94 | Medium | 05a | WP-34 | `rust/web/src/auth/server.rs:31-48` (+1 site) | No rate limiting exists anywhere in `rust/web`, yet two doc comments assert a per-IP limit as the design justification for the global advisory-lock cap and for not throttling confirm. | 00-STATE: confirmed - no rate-limiting middleware anywhere in `rust/web`. Enables the global-cap lockout lever and F-89. |
| F-95 | Low | 05a | WP-34 | `rust/web/src/auth/server.rs:1621-1652` | The WP-35 F1 concurrency test asserts `attempts >= CAP` - a lower bound where the spec prescribed an upper bound - because the prescribed bound is unachievable under the design the same spec mandated. | 00-STATE: FOURTH confirmed instance of systemic pattern 4b (tests/docs adjusted to agree with code). Escalation: the acceptance criterion was quietly renegotiated by the implementation. |
| F-96 | High (report) / DOWNGRADED (00-STATE) | 05a | WP-35 | `rust/web/src/main.rs:40-45` (+2 sites) | No manifest anywhere provisions `TURNSTILE_SECRET_KEY`, so the startup `panic!` added by WP-35 crash-loops the next prod web rollout. | **CONFLICT - 00-STATE wins:** resolved out of band, NOT a code defect. Panic is gated by `ALLOW_INSECURE_DEFAULT_KEY`, already set in `k8s/dev/web-patch.yaml:18-19` and `scripts/rust-test.sh:64`; Turnstile fails closed on every error path (`auth/server.rs:256-277`); sole fail-open (`secret.is_empty() -> true`) is what the panic prevents. "Dev default plus log warning" premise FALSE for `rust/web` - house pattern is panic-unless-opt-in (`crypto.rs:56-75`) and `docs/CODING.md:701` forbids the dev-default pattern. Remains a pre-rollout DEPLOYMENT blocker (no prod manifest sets the var). Also 00-STATE pattern 4d. |
| (no F-number assigned) | unstated | 05a | WP-35 | `rust/web/src/auth/server.rs:280-283` (+2 sites) | `TURNSTILE_SITE_KEY` has no startup check and silently defaults to empty, rendering no widget and rejecting every login - setting only the secret key is a total login outage. | New finding from `F-96-turnstile-key.md`; fold into the corpus. F-96 deployment-checklist family; both vars must land together. |
| (no F-number assigned) | unstated | 05a | - | `rust/bot/src/crypto.rs:66-76` | `rust/bot/src/crypto.rs` falls back to the hardcoded dev key with only a `tracing::warn!` and no gate in any environment, including prod. | New finding from `F-96-turnstile-key.md`; a real `docs/CODING.md:701` violation and another instance of the bot/web crypto divergence. Route to Unit 10 with F-90. |
| F-97 | Medium | 05b | WP-37 | `rust/web/src/admin.rs:254-262` (+2 sites) | `validate_provider_url` only checks the `http(s)://` prefix, so `test_provider`/`test_bot_provider` are an admin-triggered read-SSRF echoing in-pod responses, including `/metrics`. | ws F23 hardened the response handling but nothing constrains the upstream; defeats stated `/metrics` containment (`rust/web/src/main.rs:195-198`). |
| F-98 | Medium | 05b | WP-37 | `rust/web/src/admin.rs:515-533` (+1 site) | `api_key` is the only user string on the admin provider surface with no validation: empty keys are encrypted, stored, shown as `(set)` by `mask_api_key`, and sent as a bare Bearer; no size bound. | ws F25's validation sweep skipped it inside the very commit that added the helpers. Systemic pattern 2. |
| F-99 | Medium | 05b | WP-41 | `rust/web/src/db/mod.rs:161-256` | ws F35's 27-untested-function gap was closed by one smoke test naming 22 functions and asserting only degenerate/empty/negative cases, leaving behaviour unpinned. | Doc comment self-describes as "a *reminder*, not a mechanism". `is_user_admin` true path untested. Pattern 4b/4c instance. |
| F-100 | Low | 05b | WP-41 | `rust/web/src/db/mod.rs:119-159` | `session_token_validation` back-dates a token 40 days and asserts it is still valid, pinning the absence of server-side session expiry as intended behaviour. | Fifth confirmed pattern 4b/4c instance; belongs in process-fixes. Low only because `tower_sessions` store expiry also gates (05a F-85/F-86). |
| F-101 | Medium | 05b | WP-38 / WP-39 | `rust/web/src/game/mod.rs:329-355` (+1 site) | A transient `bot.command` failure leaves the message unacked, so redelivery waits the full 5-minute `ack_wait`, stalling the bot turn (and every deploy restart burns a delivery). | ws F58's `ack_wait` raise and wd F5's "leave unacked" never reconciled across work packages. Coupled to F-109 (no drain on SIGTERM). |
| F-102 | Medium | 05b | WP-39 | `rust/web/src/nats.rs:121-179` | `ensure_stream_and_consumers` only warns on stream/consumer config drift and then uses the server's values, so existing durables keep the pre-fix `ack_wait`/`max_deliver`. | Makes ws F58's fix inert on deployed environments; a server-side `max_deliver` < 3 strands messages before the code's Term (`game/mod.rs:330`), the exact wd F5 stranding. |
| F-103 | Low | 05b | WP-82 | `rust/web/src/db/mod.rs:94-101` | `create_pool` panics via `expect("DATABASE_URL must be set")` from a `Result`-returning fn and takes every `PgPool` default (max 10 conns, no timeouts) for the whole monolith. | Pre-existing; WP-82 moved it verbatim. Flagged so Unit 08 (query performance) has the pool sizing. |
| F-104 | High | 05b | WP-38 | `rust/web/src/db/bots.rs:57-71` (+5 sites) | `validate_bot_slots` matches `bot_name` case-insensitively and stores it verbatim, but every consumer resolves it case-sensitively, so `"EASY"` creates a permanently wedged game the WP-38 sweep refuses to rescue. | **00-STATE: ONE defect across FOUR units - remediate as a SINGLE item with F-138 (07), F-183 (09c), F-189 (10a).** Email `new` lowercases the bot name (`email/commands.rs:82-93`, written `:398-401`); bot lookup case-sensitive at `bot/src/config.rs:28` **and `:67`** (second site found by F-189, never cited in 05b); `bot/src/main.rs:186-194` returns `Ok(())`, acking and DISCARDING the turn. Precondition: `admin::create_bot` (`admin.rs:293-303`) permits arbitrary casing. Fix = canonicalize inside `validate_bot_slots` and return the canonical name. F-185 is the decoy all-lowercase-fixture test and must be re-fixtured in the same change. 00-STATE systemic pattern **4f** (a test blessing the lenient half of a cross-boundary inconsistency: `validate_bot_slots_accepts_case_mismatch`); 05b calls it pattern 4b - 00-STATE wins. |
| F-105 | Medium | 05b | WP-38 | `rust/web/src/game/mod.rs:200-255` (+2 sites) | `bot.turn` publishes carry no `Nats-Msg-Id` and the BOT stream has no `duplicate_window`, so three independent publishers amplify one turn into up to four LLM completions and four command attempts. | WP-38 spec's "re-publishing is safe" reasoning covers state safety but is silent on cost; conflict path deliberately spends three more completions up to `MAX_TURN_ATTEMPTS`. |
| F-106 | Low | 05b | WP-37 | `rust/web/src/admin.rs:787-813` | `read_capped_body` discards the `reqwest::Error` entirely, so reset/TLS/timeout are indistinguishable in the one admin tool whose purpose is diagnosing a misconfigured provider, and nothing is logged. | Every other failure in the file gets an `internal(context)` breadcrumb. |
| F-107 | Low | 05b | WP-38 | `rust/web/src/nats.rs:21-25` (+2 sites) | Three comments describe the term ceiling and stranded-message recovery as "(future)" WP-38/D-5 work after WP-38 shipped it (`game/mod.rs:330`), understating what depends on `MAX_DELIVER`. | Cosmetic; these are the comments a reader consults before touching ack semantics. |
| F-108 | Medium | 05b | WP-38 | `rust/bot/src/nats.rs` (+1 site: `rust/web/src/nats.rs`) | `BotTurnEvent`/`BotCommandEvent` and the subject/consumer constants are copy-pasted into the bot crate with no shared crate, no round-trip test and no `#[serde(default)]`, so a one-sided field addition is a runtime deserialization failure. | 00-STATE: not yet diverged - latent. `BotCommandEvent::attempt`'s echo invariant is documented only in the web copy (`rust/web/src/nats.rs:40-44`). **Remediate together with F-90 (Unit 05a); Unit 10 owns both.** Duplicated-module sweep DONE: exactly these two duplicates; `bot/config.rs` vs `web/config.rs` share only a filename. |
| F-109 | High | 05b | WP-36 | `rust/web/src/websocket.rs:78-80` (+2 sites) | `efad81f` deleted WP-36's ws F55 shutdown drain (`TaskTracker`, `drain_ws_tasks`, bounded 5s wait) and its regression test `rust/web/tests/websocket_hygiene.rs` together, leaving detached SSE spawns with nothing bounding the drain. | 00-STATE systemic pattern **4e** (NEW): a landed, tested fix silently reverted by a later commit in the same programme; checklist row and both commits still read as closed. Sign-off rule: assert each closed finding's citation or regression test still exists. Sharpened by F-147 (Unit 07b) - a citation must be *reachable*, not merely present - and by Unit 08 - a regression test must actually CALL the function under test. **00-STATE settles: `efad81f9` contains exactly ONE pattern-4e instance (F-109), enumerated not asserted; WP-84 spec §3g anticipated the deletion and required a proof test which does exist - so remediation is a bookkeeping fix on WP-36's row plus a decision on the never-implemented second half of ws F55 (bot consumer and email sweep tasks get no shutdown signal at `rust/web/src/main.rs:72-103`), NOT a revert of `efad81f9`.** |
| F-110 | Low | 05b | WP-37 | `rust/web/src/admin.rs:1688` (+2 sites) | Not a defect - WP-37's inline ws-finding citations are the only sign-off trail; the three apparently uncited findings (ws F24, F32, F33) were all in fact fixed, giving 14 of 14. | Recorded as the mechanism that made the review cheap; only Unit 05 work package to deliver in full. Contrast with F-109, which shows what happens where no such trail exists. |
| F-111 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:394-399` (+2 sites) | `concede_game_replace` calls `pick_replacement_bot` on the pool before `pool.begin()`, so every rejected concede commits an orphan `game_bots` row. | Salvaged from attempt 3 (unit done on 4th attempt). Escalation: `UNIQUE (game_id, name)` + no `ON CONFLICT` makes retry fail permanently as redacted internal error. Pattern 4b, fifth instance - spec's `game_bots` assertion dropped from the test (see F-114). |
| F-112 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:387-426` (+1 site) | `concede_game_replace` never updates the `games` row, so its `updated_at` claim never fails on replay: a duplicated concede swaps in a second bot and writes a second public log line. | Salvaged from attempt 3. Pattern 2 - `undo_game` got the equivalent in-transaction re-verify (Task 2.3), `concede_game_replace` did not. Remediation also closes F-113. |
| F-113 | Medium | 06 | WP-40 | `rust/web/src/game/server_fns.rs:945-947` (+1 site) | `concede_core` enforces "already left" only against the pool snapshot of `left_at`; neither the claim nor `concede_game_replace` re-checks it in-transaction. | Salvaged from attempt 3. Same for `count_active_humans` (`:819-824`). Violates the spec's own "a check against a snapshot is not a guard" rule. Feeds `left_at`-conflation coverage gap with F-116/F-117. |
| F-114 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:1816-2282` | Three of the seven new guard tests landed with spec names and error-type assertions but dropped the spec's "nothing was destroyed" state assertions. | Salvaged from attempt 3. Pattern 4b. Sites: `:1889`, `:1816`, `:2178` plus `concede_game_requires_two_players`. The dropped `game_bots` assertion is the one that fails today (F-111). |
| F-115 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:348-355` (+1 site) | Task 6's "unreachable" error is gated by `active_humans == 2` while `concede_game` counts all `game_players` rows, so a 3-player game with an eliminated player hits a redacted internal error and concede is permanently impossible. | Salvaged from attempt 3. Divergence path pinned by `elimination_sets_left_at_once` (`game_write.rs:1291`); F-116 reaches the same failure mode without anyone leaving. |
| F-116 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:584-598` (+2 sites) | `undo_game`'s `left_at` CASE has no arm for un-elimination, so an undone elimination permanently marks the player a leaver and `compute_ranked_placings` rates them last. | 00-STATE: clean instance of systemic pattern 2 - WP-40 added `AND NOT $9` to the byte-identical sibling CASE in `update_game_command_success` (`:743-744`) and left `undo_game`'s copy alone. Clause pre-dates `9ba3736b`; sweep confirms nothing anywhere sets `left_at` back to NULL. |
| F-117 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:356-381` (+2 sites) | `concede_game` rates a finished game without calling `write_ranked_placings`, leaving `ranked_placing` NULL on every conceded game while the other two finish paths populate it. | Pattern 4f: `placing.rs:106-127`'s `two_player_concede` test fixtures the conceder with `left_at: Some(..)`, a state `concede_game` never produces. Shares `left_at`-conflation root cause with F-113/F-116. |
| F-118 | Low | 06 | WP-40 | `rust/web/src/db/game_write.rs:584-598` (+2 sites) | `undo_game` does not restore `game_players.points`, so points stay at their post-undone-move value; `end_game` orders placings by that stale column. | Low: narrow path (`end_game` right after an undo with no intervening command). Feeds directly into F-120's unguarded `ORDER BY points`. |
| F-119 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:401-410` (+2 sites) | `concede_game_replace` clears `is_turn` unconditionally without reassigning the turn, so conceding on your own turn wedges the game - `find_bot_turns` returns zero rows and the replacement bot never plays. | 00-STATE: unit 06's ONE open dependency. At unified-report time cross-check WP-38's bot-turn wedge-recovery sweep (Unit 05); if it gates on `is_turn` rather than re-deriving from the game service, F-119 has NO production mitigation. Severity stated on the `is_turn`-gating assumption. |
| F-120 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:430-473` (+2 sites) | `end_game` is a fourth lifecycle writer that reads then writes and ends in an irreversible rating write with no `expected_updated_at` and no claim, violating the `docs/CODING.md` rule shipped in the same commit. | 00-STATE: systemic pattern 4b's mirror-image variant - not a test edited to agree with code, but a new doc rule scoped to three named functions so `end_game` is invisible to the grep procedure the doc itself prescribes. Must be named in the unified report's process-fixes section. |
| F-121 | Low, informational | 07 | - | `rust/web/src/bin/import_game.rs:20-32` | The 100 MiB import guard `stat`s the file then reads unbounded, so a FIFO or `/dev/stdin` reports len 0 and bypasses it, and the byte cap bounds no row counts in `import_bundle`. | CF-3. Dev-only CLI; operator supplies the path, so nothing attacker-controlled. |
| F-122 | Low, informational | 07 | - | `rust/web/src/game/import.rs:109,124` (+1 site) | `import_bundle` writes bundle-supplied `undo_game_state` verbatim with no validation and `undo_game` later replays it after only a non-NULL check. | CF-4 from Unit 06; downgraded - no HTTP route, no `#[server]` fn, sole caller is the dev CLI `bin/import_game.rs:35`. Reachability independently confirmed in Verified good (`router.rs:147-148`). |
| F-123 | REFUTED | 07 | WP-42 | `rust/web/src/visibility_cache.rs:12-13,25-31,58` | Claimed `VisibilityCache` cross-user visibility leak because the key omits the viewer while `is_proposal_visible_to_user` is user-scoped. | **REFUTED (report + 00-STATE agree, do not re-derive):** each instance is a plain local inside the per-request spawn at `events.rs:65`; one instance = one connection = one viewer. Archived text retained in report. Same pass also refuted the claim that WP-42 was reverted by the SSE migration - a useful negative against pattern 4e. Downgraded remnant is F-132; secondary owner-visibility half became F-133. |
| F-124 | High | 07 | WP-50 | `rust/web/src/proposals.rs:1730-1786` (+2 sites) | `add_proposal_player` passes `email` raw to `find_or_create_user_by_email_tx` and `check_invite_policy_tx` - no canonicalize, no empty/`@` check - so `" foo@x.com "` mints a verified ghost account and bypasses D7 block-by-target and `invite_policy`. | Unit's canonical instance of checklist-satisfied-literally: WP-50 spec 3c enumerated only `create_proposal` and `restart_game_with_roster`. Exploitable only because F-125's index omits the trim half. **00-STATE remediation proposal: a `CanonicalEmail` newtype whose only constructor is `canonicalize_email` would permanently close this class - the contract is today enforced only by doc comment. Fold into ONE item with F-127, F-128 and F-173.** |
| F-125 | Medium | 07 | WP-50 | `rust/web/migrations/026_canonical_emails.sql:33` | The unique index is on `lower(email)` while the backfill one line above normalizes with `lower(btrim(email))`, so trim-variant duplicates coexist and no `CHECK` enforces canonical storage. | The enabling half of F-124 (whitespace variant lives; case-only variant merely 500s). `CHECK (email = lower(btrim(email)))` is the stronger fix. |
| F-126 | Medium | 07 | WP-50 | `rust/web/src/proposals.rs:1733 -> :1772 -> :1191` | `add_proposal_player` is the only caller not honouring WP-50's "callers validate emptiness" contract: `email: Some("")` reaches `INSERT INTO user_emails ... VALUES ($1, '', true, NOW())`, creating a junk verified account then 500ing on the 23505. | Every other path confirmed rejecting (`login:296`, `confirm_login:349`, `add_email_address:856`, `create_proposal:1388`, `restart_game_with_roster:1293`, both client boundaries). Same fn as F-124. |
| F-127 | Low | 07 | WP-50 | `rust/web/src/db/game_write.rs:81-115` | `create_game_with_users_tx` resolves `opts.opponent_emails` by exact match and inserts the raw string, and is the one db helper that never got WP-50 criterion 3a's "callers must pass canonicalized addresses" doc comment. | Latent - all thirteen production callers pass `&[]`. `db/emails.rs:71` and `db/visibility.rs:171` got the comment. **00-STATE remediation proposal: `CanonicalEmail` newtype (only constructor `canonicalize_email`) closes this class permanently; contract is currently doc-comment-only. Fold into ONE item with F-124, F-128, F-173.** |
| F-128 | Low, note | 07 | WP-50 | `rust/web/src/email/inbound.rs:538` (+3 sites) | Canonicalization is Rust full-Unicode `to_lowercase` (`auth/email_addr.rs:3-5`) while the unique index (`026:33`) and the inbound authorization compare use Postgres `lower()`; inbound `extract_addr_spec` (`:134,150`) trims but never lowercases. | **CONFLICT - 00-STATE wins:** report records this as a fails-closed negative result; 00-STATE (via Unit 09b's F-173) says **F-128 is NOT closed and has NO owner** - `from_matches_verified_email` compares in SQL (`LOWER`) while every write path canonicalizes in Rust, and `İ@example.com` breaks. F-173 strengthens the `CanonicalEmail` newtype proposal. **Fold F-128, F-173 and the F-124/F-127 newtype proposal into ONE remediation item.** |
| F-129 | Medium (report) / **ESCALATED - ACCOUNT TAKEOVER** (00-STATE) | 07 | - | `rust/web/src/email/inbound.rs:520-530` (+2 sites) | The `s-{token}@brdg.me` settings-email token has no expiry, no rotation and no revocation - `ensure_settings_email_token` returns the same value forever and nothing NULLs it on use, logout or email removal, so any archived settings email is a permanent live bearer credential. | CF-2. **00-STATE: Unit 07 set an escalation condition and it FIRED.** F-161 (High, Unit 09a) escalates F-129+F-130 to account takeover: WP-56's inbound auth gate is fail-open three independent ways (cleanest: `spf=fail; dkim=none` -> `Pass`, because the code requires SPF *and* DKIM to both say "fail", inverting the DMARC rule); combined with this token's lack of expiry/single-use/rate-limit, spoofing `From:` is account takeover. **Session's most severe finding - top of the unified report's remediation order. In-report Medium is superseded.** Pattern 2 within one subsystem: `unsubscribe_token` rotates (`email/unsubscribe.rs:99`), invite tokens rotate (`proposals.rs:936-944`), this one does not. |
| F-130 | Medium (report) / **ESCALATED - ACCOUNT TAKEOVER** (00-STATE) | 07 | - | `rust/web/src/email/commands.rs:329-346` (+2 sites) | The "settings" token is not scoped to settings: a holder reaching `dispatch_standalone_server_command` also gets `new` (create a real game naming arbitrary opponents and bots), `bump` and subscribe/unsubscribe. | **00-STATE: ESCALATED with F-129 - see that row.** The report's sole mitigating control, `from_matches_verified_email` + SPF/DKIM/DMARC (`inbound.rs:1421-1433`, `:191-214`), is exactly what F-161 shows is fail-open, so the Medium rating's stated precondition no longer holds. In-report severity superseded. |
| F-131 | Low/Medium | 07 | WP-42 | `rust/web/src/events.rs:33-41` | SSE streams call `validate_session_token` exactly once at connect and never again, so after logout or session revocation the stream keeps delivering frames indefinitely - only visibility is refreshed (30s TTL), never authentication. | 00-STATE: routed to **Unit 09 for confirmation**. Adjacent to Unit 09's ownership of `efad81f`; raised here because the visibility work is WP-42's. |
| F-132 | Low | 07 | WP-42 | `rust/web/src/visibility_cache.rs:11` | `VisibilityCache` keys on an id alone, correct only because each instance is owned by exactly one SSE task with one fixed viewer - an invariant nothing in the type expresses. | Downgraded remnant of the REFUTED F-123 (see that row); 00-STATE agrees the leak is refuted. Fix: doc the per-viewer ownership requirement or key on `(id, Option<Uuid>)`. |
| F-133 | Low | 07 | WP-42 | `rust/web/src/db/proposals.rs:40-52` | `is_proposal_visible_to_user` grants visibility only via a `game_proposal_players` row and never consults `game_proposals.owner_user_id`, so an owner not also inserted as a player cannot see their own proposal. | Secondary half salvaged from the REFUTED F-123 (see that row). Untested in either direction - both tests (`:172`, `:193`) add the owner as a player explicitly. |
| F-134 | High | 07 | WP-79 | `rust/web/src/proposals.rs:1702-1709` | `start_proposal` calls `fetch_game_from_service` (a `reqwest` call) at `:1702` while still holding the `lock_proposal_for_update` `FOR UPDATE` row lock taken at `:1652`, so a hung game service blocks every concurrent respond/cancel/transfer/nudge. | Hoisting exactly this call is WP-79's whole point; done in `create_proposal` (`:1105`) and `restart_core` (`game/server_fns.rs:1091`), not here. The commit message reads clean. WP-79's own breakdown gotcha coming back positive. |
| F-135 | High | 07 | WP-79 | `rust/web/src/email/inbound.rs:1021-1034` | `91c723d4` - the WP-79 commit itself - inserted `fetch_game_from_service` at `:1022`, after the `begin()` at `:922` and the lock at `:931`, so the refactor moved the call out of `start_proposal_tx` but landed it on the wrong side of `begin()`. | Sharpest checklist-satisfied-literally instance in the unit. Harder to hoist than F-134 (`accepted_count` depends on the in-tx response UPDATE). Inbound-webhook path also holds the lock across the whole render. |
| F-136 | High | 07 | WP-46 | `rust/web/src/email/sweep.rs:135-137` | The `_ =>` catch-all after `fetch_email_recipient` swallows `Err(_)` as `PermanentSkip`, which `sweep_once` (`:289-305`) treats identically to `Sent` - so one transient DB error commits `mark_reminder_sent_tx` and the reminder is never sent. | Reintroduces the wfe F30 mark-without-send that WP-46 exists to remove; spec says errors are `Retry`. **00-STATE: the High-severity web-half instance of systemic pattern 5 (`_ => <default>` substitution, cf. F-65) - promote in the unified report, no longer a game-crate curiosity.** **Remediation pairing: fix with F-145 (Unit 07b) in the SAME change** - F-136 lives in the surviving duplicate of the abandoned wfe F36 dedup. |
| F-137 | High | 07 | WP-45 | `rust/web/src/game/server_fns.rs:1087` | `restart_core` takes client-supplied `bot_slots` from `restart_game_with_roster` (`:1271`, `:1299`, `:1334`) and never calls `validate_bot_slots`, so a restart carrying `bot_name: "garbage"` reaches `insert_game_from_service` and creates a wedged game. | WP-45 spec section 1 names `restart_core` as one of the three wd F27 call sites; `rg validate_bot_slots` has zero hits in the file. Solo-vs-bots branch (`:1178`) unguarded; multi-human branch saved only incidentally by `proposals.rs:1411`. |
| F-138 | Medium | 07 | WP-45 | `rust/web/src/db/bots.rs:61-63` (+4 sites) | `validate_bot_slots` matches with `n.eq_ignore_ascii_case(&slot.bot_name)` and neither returns nor imposes a canonical name, so all four entry points persist the client's string and no case-sensitive consumer will ever match it. | **00-STATE: closes the loop on Unit 05b's F-104 from the write side - ONE defect spanning FOUR units; remediate F-104 + F-138 + F-183 (09c) + F-189 (10a) as a SINGLE item.** Fix = canonicalize inside `validate_bot_slots` and return the canonical name. F-185 is the decoy test that hid it and must be re-fixtured in the same change. Entry points: `proposals.rs:1264`, `:1812`, `:1411`, `email/commands.rs:420`. |
| F-139 | Medium | 07 | WP-48 | `rust/web/src/game/import.rs:190-210` | The wd F10 unique-violation fallback runs `generate_unique_username` and a second INSERT on the same aborted import transaction (`placeholder_user` called at `:103` with `&mut tx`, no SAVEPOINT), so it always fails with 25P02 and only changes the error text. | Guard is present and satisfies its checklist row while changing nothing. Capped at Medium because the path is the dev-only CLI. Fix: nested savepoint before the retry, or a separate connection. |
| F-140 | Medium | 07 | WP-49 | `rust/web/src/db/game_types.rs:81-91` (+1 site) | `find_game_version_rules` / `find_game_version_render_meta` filter on `is_public = true AND is_deprecated = false`, but `run_rules` resolves the version from the game, so a player in an in-flight game on a deprecated version gets "Game version not found". | Consumed at `rust/web/src/email/commands.rs:939-946`; same breakage for `/rules/<version_id>` links from `email/notify.rs::rules_url`. Spec asked only for the public-page filter and never considered the by-game callers; public page itself verified correct. |
| F-141 | Low | 07 | WP-46 | `rust/web/src/proposals.rs:973` (+2 sites) | Rider wfe F40 required a LIMIT on all four sweep candidate queries; `fetch_nudge_candidates`, `fetch_expiry_candidates` and `fetch_auto_decline_candidates` (`:973`, `:1007`, `:1082`) are still unbounded - only `email/sweep.rs:44-53` got one. | Systemic pattern 2 (one of a set hardened, siblings left). Fix: apply the same shared const limit to all three. |
| F-142 | Low | 07 | WP-48 | `rust/web/tests/ssr_pages.rs:1290-1300` | `admin_export_route_rejects_non_admin` uses a fresh `Uuid::new_v4()` as the game id, so the spec's "body must not contain the private log body" assertion is vacuous - there is no game to leak. | **00-STATE: a confirmed "Test? y checklist row with no real test" instance; Unit 08 elevates this to the session's most-confirmed pattern (with F-148, F-149, F-150). Process fix: grep the checklists for "Test? y" rows and confirm a test exists for each.** Minor framing difference: report says the test exists with one vacuous assertion (the 403 half is real); 00-STATE files it under no-test-exists. Fix: seed a real game with a private log. |
| F-143 | Low, note | 07 | WP-46 | `rust/web/src/email/sweep.rs:260-306` | By WP-46 spec 3a the claim transaction holds a `game_players` `FOR UPDATE` lock across `send_reminder` - a game-service render plus the Resend API call - serialised over up to 200 candidates per tick. | Not a deviation from its own spec; recorded so WP-46 and WP-79 are not read as contradicting each other (WP-79 removes network-calls-under-lock on the proposal path while WP-46 mandates it on the sweep path). Unified report should reconcile into one policy. |
| F-144 | High | 07b | WP-46 (`69bcd1e`) - NOT WP-51 | `rust/web/src/email/sweep.rs:507-519` (+2 sites) | Invite-nudge dedup key `gp.nudged_at` is per-proposal while sends are per-invitee, so one web-suppressed invitee blocks the mark and re-nudges the whole roster every tick. | **00-STATE attribution: WP-51 (`dcd8844c`) introduced none of F-144/F-145/F-146** - this is WP-46's code, first review pass over it. Live duplicate-email bug (~1,344 dupes/invitee over the 14-day expiry at the 900s interval), not a nit. Carry to whoever owns the invite/proposal email surface. |
| F-145 | Medium | 07b | WP-46 (`69bcd1e`) - NOT WP-51 | `rust/web/src/proposals.rs:257-296` (+1 site) | `send_invite_core` folds `Err(_)` into `Ok(None)` at three `let-else` returns, so a transient DB failure returns `true` ("permanently unsendable"), marking the proposal nudged with nothing sent. | **00-STATE: must be fixed in the SAME change as F-136 (Unit 07)** - F-136 is the High-severity instance of systemic pattern 5 (`_ => <default>` substitution) living in the surviving duplicate of the abandoned wfe F36 dedup; same defect class in the two halves of one sweep module. **Attribution (00-STATE): WP-51 introduced none of F-144/F-145/F-146**, though WP-51 rewrote these exact three lines for wd F34 without fixing them. Third pattern-5 instance in the web half (F-65, F-136, F-145). |
| F-146 | Low/Medium | 07b | original #24 invite work (`4bd3135`/`db8f4b6`/`b88ff26`) - NOT WP-51 | `rust/web/src/proposals.rs:401` (+8 sites) | Five distinct proposal notifications (reinvite, decline, cancelled, started, ready) all render subject `"{game_type_name} invite"` and thread id `proposal-{id}`, collapsing actionable mails into one hidden conversation. | **00-STATE attribution: WP-51 introduced none of F-144/F-145/F-146.** Violates the de-threading house rule stated at `notify.rs:88-94` and applied on the turn path. Capped below High: nothing dropped, threading is client-dependent. |
| F-147 | Medium | 07b | WP-51 (`dcd8844c`) - WP-51's own | `rust/web/src/email/notify.rs:523-543` (+1 site) | wfe F36's dedup was consciously abandoned, but `notify::send_turn_reminder` shipped dead-at-birth with a doc comment stating the dedup as accomplished fact and the checklist records wfe F36 closed. | **00-STATE: sharpens F-109's sign-off rule** - a closed finding's citation must be *reachable*, not merely present; `send_turn_reminder` exists, has never had a caller, and its doc comment defeats F-109's check as originally written. Purest instance of pattern 1 (routing leak). Also a live trap: uses `SendMode::Normal` (turn opt-out) for a reminder. **00-STATE REFUTED, do not re-derive: there is no pattern-4e revert in `dcd8844c`** - it edited `sweep.rs::send_reminder` in place. F-147 + F-136 are one remediation item. WP-51 DOES have a spec (`planning/specs/WP-51-invite-mailer-notify-dedup.md`, Tier-2); WP-53 has none. |
| F-148 | Medium | 07b | WP-53 (`3610b957`) | `rust/web/src/db/game_write.rs:739` | `wd F6`'s `CASE WHEN $9` elimination guard is correct but wholly unpinned - deleting it fails no test in the repository, on a row the checklist marked "Test? y". | **00-STATE: one of four confirmed "Test? y checklist row with no test actually existing" instances (with F-142 (07), F-149, F-150 (08)); Unit 08 elevates this to the session's most-confirmed pattern, to be a top-level systemic pattern in the unified report.** Explicitly NOT pattern 4b - fix correct, commit clean, row honestly closed, guard simply unpinned. Process fix: grep checklists for "Test? y" rows and confirm a test exists. |
| F-149 | Low | 07b | WP-53 (`3610b957`) | `rust/web/src/friends.rs:229-231` | `wd F61`'s required test is absent and `friends.rs` (634 lines) has no `#[cfg(test)]` module at all, so nothing asserts `block_user`'s "User not found" guard. | **00-STATE: same "Test? y with no test" cluster as F-142, F-148, F-150** - session's most-confirmed pattern. Lead's recorded decision (`EXECUTION-STATE.md:175`) excuses only the *integration* test on "db layer already tested", but the new guard is not in the db layer. Code itself correct; TOCTOU unreachable (no `DELETE FROM users` in `rust/`). Only `wd F25` of WP-53's three "Test? y" rows got a test. |
| F-150 | Medium | 08 | WP-52 | `docs/reviews/2026-07-23-rust-review/planning/checklists/T3-B5-web-domain-stats-misc.md` (+7 checklist rows) | All seven `Test? y` rows of WP-52 shipped with no test - `f374434d` touches no test file and adds no `#[cfg(test)]` block, while four of the seven change query result semantics. | **00-STATE: with F-142 (07), F-148 and F-149 (07b) this is the fourth and largest confirmed "Test? y checklist row with no test actually existing" instance - the session's most-confirmed pattern, to be elevated to a top-level systemic pattern in the unified report.** Aggravated by a pre-existing `#[sqlx::test]` module + fixtures at `rust/web/src/stats/queries.rs:762-2287`. A `wd F48` test would have caught F-151. Also: WP-52 has NO spec - extend 00-STATE's no-spec list (WP-24, WP-27, WP-44, WP-53, WP-79). |
| F-151 | High | 08 | WP-52 | `rust/web/src/stats/queries.rs:104-152` (+1 site) | `wd F48`'s game-type name filter is applied only inside the `qualifying` CTE and not to the `gtu` side of a FULL OUTER JOIN, so `.next()` on the alphabetically-ordered result returns another game type's rating and record. | Public unauthenticated endpoint (`viewer_user_id: None` valid); silent wrong data for any player rated in >1 game type. Regression is the pairing of the half-applied filter with the caller's `.find` -> `.next` switch at `rust/web/src/stats/mod.rs:266-279`. Also renders the `.unwrap_or_else` zeroed fallback at `:271-279` near-unreachable. The `wd F48` test F-150 records as missing would have caught it. |
| F-152 | Medium | 08 | WP-52 | `rust/web/src/stats/queries.rs:713` | `wd F55`'s `NULLS LAST` fix skipped `recent_form_for_game_type`, whose byte-identical `finished_at DESC` window still sorts NULL-`finished_at` legacy rows to `rn = 1`, displacing recent games on the game-type leaderboard. | Systemic pattern 2 (inconsistent hardening within one file) - web-half instance beside F-61 and F-116. Aggravating: the same commit edited this function for `wd F50` (`:700-701`). Named sites fixed at `:312` and `:638`; `rating_series:195`, `game_history:455` unaffected. |
| F-153 | Medium | 08 | WP-52 | `rust/web/src/stats/queries.rs:7-20` | `wd F50`'s "one `const` used by all eight sites" shipped as an `#[allow(dead_code)]` string with ZERO referents and a doc comment stating manual sync is now required - nine hand-synced copies instead of eight. | **00-STATE: NEW named pattern, "the documentation-only constant"** - a row asking for extraction to a shared definition satisfied by creating the definition and touching no call site. Relative of pattern 5 but distinct: it leaves a greppable `#[allow(dead_code)]` marker. **Sign-off action: sweep `rg "allow\(dead_code\)"` across the commit range.** Durability finding only - nine copies verified in sync today. Stated `sqlx::query!` blocker is real but `macro_rules!` + `concat!` would have satisfied the row. |
| F-154 | Medium | 08 | WP-52 | `rust/web/src/stats/mod.rs:343-348` | `wd F52`'s canonicalization binds `find_game_type_name`'s `None` for an unknown game type straight into the `($3::text IS NULL OR gt.name = $3)` predicate, so an unknown filter returns the player's entire history instead of nothing. | Fails the row's one explicit criterion - parity with `get_player_game_type_stats`, which 404s at `stats/mod.rs:258-264`. `filters.game_type` at `:384` then tells the client no filter was applied. Second result-semantics change in this commit with an unfilled `Test? y` box - see F-150. |
| F-155 | Low | 08 | WP-52 | `rust/web/src/stats/queries.rs:511` (+2 sites) | `wd F53`'s justifying comment is copy-pasted verbatim to three `query_as` sites and is factually wrong at `game_history_count`, whose destination is the tuple `(i64,)`, not a named `FromRow` struct; "binds are static" argues FOR the macro it declines. | Third row in this commit satisfied by an artifact rather than the effect (with F-153, F-150). Exactly the F-147/F-109 hazard: the citation exists at all three sites and a sign-off grep marks the row closed. Sites `:232`, `:429-430`, `:509-510`. |
| F-156 | Medium | 08 | WP-52 | `rust/web/src/index.rs:47-73` | `wd F74`'s `take(20)` bound is applied to `list_friends`' `ORDER BY lower(u.name)` output, so the home page's friends-recent-games feed is truncated alphabetically - friends sorting after the 20th are permanently invisible with no UI indication. | Milder form of F-151's class: a perf row that changed which subjects the feature covers. Concurrency half correct (`try_join_all`), though it fires up to 20 simultaneous pooled queries per render - `buffer_unordered` suggested in the same change. `list_friends` at `rust/web/src/db/social.rs:205-217`. |
| F-157 | Low | 08 | WP-52 | `rust/web/src/friends.rs:100-108` (+1 site) | `tokio::try_join!` rewrites collapsed eleven per-query `internal(...)` error contexts into two catch-alls, so on-call cannot tell which of six friends queries or five game-info queries failed. | Observability regression only; no behaviour change and no row prohibited it. Second site `rust/web/src/game_info/mod.rs:164-172`. `wd F62`/`wd F75` otherwise satisfied correctly. Fix: keep per-future `.map_err` inside the `try_join!` arguments. |

## Severity tally

- High: 14
- Medium: 32
- Low: 17
- High (report) / DOWNGRADED (00-STATE): 1
- Medium (report) / **ESCALATED - ACCOUNT TAKEOVER** (00-STATE): 2
- Low/Medium: 2
- Low, note: 2
- Low, informational: 2
- REFUTED: 1
- unstated (both `(no F-number assigned)` rows): 2

Total: 75 rows (73 numbered findings F-85..F-157 + 2 unnumbered)

## Discrepancies

### Report vs `00-STATE.md` conflicts (00-STATE wins in every case)

- **F-96** - `05a` rates it **High** and treats it as a code defect. 00-STATE
  **DOWNGRADES** it: resolved out of band, *not a code defect*, retained only as a
  **pre-rollout deployment blocker** plus systemic pattern 4d. Row carries both.
- **F-104 pattern label** - `05b` calls the blessing test "pattern 4b"; 00-STATE
  names it pattern **4f** (a test that blesses the lenient half of a cross-boundary
  inconsistency). 00-STATE's label used.
- **F-104 citations** - `05b` cites `bot/src/config.rs:24-28` and `main.rs:187-190`;
  00-STATE cites `config.rs:28` **plus an uncited `:67`** (found by F-189) and
  `main.rs:186-194`. 00-STATE's citations used.
- **F-109 remediation** - `05b` prescribes restoring the `TaskTracker` drain plus a
  new SSE test. 00-STATE (settled by Unit 09a) says remediation is a **bookkeeping
  fix on WP-36's row plus a decision on the never-implemented second half of ws F55**
  - explicitly **NOT a revert of `efad81f9`**.
- **F-128** - `07` records it as a fails-closed negative result. 00-STATE (via Unit
  09b's F-173) says **F-128 is NOT closed and has no owner**; `İ@example.com` breaks
  the SQL-`LOWER` vs Rust-canonicalize divergence.
- **F-129 / F-130** - `07` rates both Medium with an explicit escalation condition.
  **The condition FIRED** (F-161, Unit 09a): both are **ESCALATED to account
  takeover**. Their in-report Medium is superseded.
- **F-142** (framing, not rating) - `07` describes a test that exists with one
  vacuous assertion (its 403 half is real); 00-STATE files it under the "Test? y row
  with no test" tally. Both recorded.

### ID sequence

- **No gaps and no duplicates across F-85..F-157.** All 73 IDs present exactly once.
  No unit in this range sub-letters a finding.
- **Ordering anomaly in `07`**: findings are presented non-monotonically
  (F-121, F-122, F-129..F-132, REFUTED block, F-133, ARCHIVED block, F-124..F-128,
  then F-134+). Extraction is by ID, not by position.
- **`07` header miscount**: its progress line (`:22`) says "13 findings, F-121..F-143";
  that range is **23** findings.
- **F-123 is REFUTED/ARCHIVED**, not a live finding. Its owner-visibility half was
  re-issued as **F-133**; its downgraded remnant is **F-132**. Kept as a row so it is
  not re-derived.
- **F-110 is explicitly "not a defect"** in `05b`; kept as a row rather than dropped.
- Severity-column heterogeneity: five rows carry compound or qualified severities
  (`Low/Medium`, `Low, note`, `Low, informational`, plus the two conflict rows). The
  composition Lead must normalize these; this extraction preserved them verbatim.
- WP attribution for F-104, F-105, F-107, F-108 and F-133 is **inferred** (05b/07
  headings carry no WP label). F-133 assigned WP-42 as the salvaged secondary half of
  the refuted F-123.

### Substantive items in these reports carrying NO F-number

These would be lost if the composition Lead works from the findings table alone.

**Deployment-checklist family (group with F-96, F-207, and 09b's `public_base_url`):**

1. `TURNSTILE_SITE_KEY` has no startup check - already promoted to a row.
2. `rust/bot/src/crypto.rs:66-76` ungated dev-key fallback - already promoted to a row.
3. One `ALLOW_INSECURE_DEFAULT_KEY` flag disables **two unrelated** production guards
   (`crypto.rs:60` and `main.rs:42`). Splitting it is suggested, not urgent.
4. `k8s/argocd/brdgme-app.yaml` is a stale duplicate (already `docs/BACKLOG.md:67`),
   and the config repo's claimed CI auto-push **does not exist** - new SealedSecret
   files get no `kubeconform` validation.

**Decoy / vacuous tests not carrying their own F-number:**

5. `verify_turnstile_rejects_on_transport_error` (`auth/server.rs:1856-1862`) makes a
   **real network call to Cloudflare** and passes for the wrong reason. No test covers
   non-200, malformed JSON, or a live `success: false`.
6. `rating_before_aggregates_exclude_nulls` (`stats/queries.rs:1287-1346`) name-matches
   `wd F51`'s risk exactly, **never calls `game_history`**, and asserts PostgreSQL
   aggregate semantics instead. **This is the source of 00-STATE's F-109 sign-off
   sharpening (ii): a regression test must actually call the function under test.**

**Coverage gaps (no F-number, no owner):**

7. No request-parts test harness - every `#[server]` fn in `auth/server.rs` is untested
   end to end. Structural cause of F-92 and why F-85 was uncatchable.
8. `rust/web/src/crypto.rs` has **no `load_key` test** - the whole ws F16 fix is
   unexercised, while the *unfixed* bot copy tests all three paths.
9. `require_admin`'s true path is untested for **13 of 16** server fns.
10. `admin.rs:1560-3488` (Leptos UI components) never read.
11. `db/game_write.rs` largely unread; `update_game_command_success` has had **no
    line-level review this session**.
12. `rust/bot/src/prompt.rs` (442 lines) and `routing.rs` had no owning sub-unit at 05b
    time. (Unit 10a later REFUTED `prompt.rs` as a leak vector but found F-192/F-193.)
13. `email/sweep.rs` verified only for the WP-38 bot-turn sweep; its test module never
    read. Other sweeps are WP-46's.
14. **WP-51 `dcd8844c` and WP-53 `3610b957` were NOT audited by Unit 07** - six
    `RealInviteMailer` methods, the `spawn_sweep` collapse, `notify_owner_decline`'s new
    gating. Unit 07b was dispatched for exactly this; confirm it closed the whole
    surface.
15. WP-46's `proposals.rs` half (+428) read only at four spec-named fns; its new test
    module unaudited.
16. No test exercises a **live game after concede-with-replace** - tests assert the
    write, never the invariant. This is why F-119 survived seven new guard tests.
17. Nothing tests `compute_ranked_placings` against a state any real finish path
    produces (its three tests use hand-built vectors). F-117 is the consequence.

**Remediation-shaping observations:**

18. **`left_at` conflates "eliminated by play" with "left the game"**, is written by four
    call sites and has no owner. **F-113, F-116 and F-117 are all symptoms - carry as ONE
    schema-change item, not four.**
19. **Three email-borne bearer tokens** (settings, unsubscribe, invite) have three
    different lifecycle disciplines, no shared abstraction, and only two rotate. Worth
    one combined remediation item.
20. The `CanonicalEmail` newtype is Unit 07's single most valuable recommendation;
    the contract is enforced only by doc comment (`db/emails.rs:71`,
    `db/visibility.rs:171`). Fold with F-128 + F-173 into one item.
21. `i-noreply@brdg.me` is not short-circuited in the inbound router
    (`inbound.rs:95-96` -> `InboundRoute::Invite("noreply")`, `:856-866` runs a real
    token lookup that misses). One wasted query per one-way mail; two-line fix.
22. `restart_core`'s pool-read-under-`FOR UPDATE` is off-convention (neighbours at
    `:1136`/`:1145` use `_tx` variants). **Not a deadlock** (00-STATE refuted), but an
    `is_player_in_game_tx` would be strictly better.
23. `wd F46`'s `page + 1` next-page-link clamp is **unverified** (one-line change to
    `rust/web/src/players.rs`); deliberately not raised, flagged for a remediation pass.
24. `wd F49` bounds the payload **in Rust after fetching every row**, so DB work stays
    unbounded on an anonymous endpoint. The checklist row's wording is satisfied;
    deliberately not raised.
25. WP-53 residual cosmetics: `3610b957` deleted `encode_path_segment`'s doc comment and
    left a mid-file `use percent_encoding::...` at `players.rs:34`; `wd F77` was
    satisfied by a two-word swap in `settings.rs:1-2` that does not enumerate
    add/confirm/make-active/remove as the row asked.

**Corrections to `00-STATE.md` these reports establish:**

26. **The no-spec list is longer than 00-STATE records.** Add **WP-36, WP-43, WP-44,
    WP-52 and WP-53** to the existing WP-24/27/44/53/79 note. WP-36 is the crypto
    package - the highest-stakes item in Unit 05 has only a commit message as its
    acceptance criteria, so that verdict rests on weaker evidence than the rest.
27. **`SUMMARY.md` at HEAD is a narrative compaction carrying NO `wd Fnn` / `wfe Fnn`
    ids.** Finding text survives only at `868094a6:.../findings/*.md`. Recon fact.
28. Unit 07's recon corrects the breakdown's file names: `db/game_visibility.rs` and
    `controller/import_game.rs` **do not exist** (`db/visibility.rs`, `bin/import_game.rs`).
29. **Unit 08 sizing in `00-breakdown.md` is wrong**: 91 of WP-52's 95 files are `.sqlx`
    cache JSON; only 9 Rust source files, +162/-80. The "mostly-deletions consolidating
    duplicated code" framing is false and the pattern-4e deletion-risk surface is near zero.
30. `05b`'s WP-82 verdict has a caveat: `db.rs` split is a pure move (21 symbols, no SQL
    change, 128 tests before and after) **but comparing against HEAD shows 24 extra tests
    added by later commits.**

**Verified-good negatives worth preserving (prevent re-derivation):**

31. WP-37 admin authz is clean - all 16 server fns call `require_admin`, with a
    source-level self-check `every_admin_server_fn_calls_require_admin`. WP-37 is the
    only Unit 05 package delivering full scope (14/14).
32. WP-38 fully delivered (3a-3d plus section-5 tests); WP-39 `supervise_consumer`
    correct; `reorder_bots` the strongest function in the unit; WP-68 completely clean
    (`term_size` gone from tree and lockfile).
33. F-91's AAD-less ciphertexts checked at all admin call sites - **not exploitable there**.
34. `rust/bot/src/main.rs` has real graceful shutdown and zero `unwrap`/`todo` - better
    than the web side.
35. `5e9bae2c` is a genuine de-flake, **not** a weakened assertion (pattern 4b suspicion
    cleared). Residual: per-log `created_at` collapse is no longer catchable.
36. WP-47 is wired end-to-end, and WP-44's guard removals are net-neutral TOCTOU
    closures - explicitly **not** pattern 2 or 4e instances.
37. `replacement_bot_available` and `pick_replacement_bot` share the same predicate -
    the F-115-shaped mismatch is **not** present there.
38. `game_bots` has `UNIQUE (game_id, name)` (`migrations/003:15`) and
    `pick_replacement_bot` has no `ON CONFLICT`. Answered, folded into F-111.
39. The only spec-prescribed test name absent
    (`concede_game_replace_rejects_stale_updated_at`) is one **the spec never actually
    asked for** - not a gap.

**Process notes:**

40. `9ba3736b` bundles 224 lines of `docs/reviews/.../planning/` state into a code
    commit - the same mixed-commit pattern the breakdown flagged for `62b293df`.
41. `05b` recommends **mandating finding-id citation at fix sites** as a process fix.
42. `08-web-stats-query-perf.md:572-578` has duplicate empty `Verified good` and
    `Coverage gaps` headings marked `_(pending)_` that **shadow the real populated
    sections above them** - a template artifact, not missing content.
43. Session continuity: Unit 06 completed on its **4th attempt** (F-111..F-115 salvaged
    from attempt 3); Unit 07b **died to quota exhaustion mid-unit before producing any
    finding** and was re-dispatched from scratch, producing all six on the re-run.
