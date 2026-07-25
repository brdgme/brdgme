# Raw findings: web domain — game/mod.rs, game/export.rs, game/import.rs

Scope: `rust/web/src/game/mod.rs` (1101 lines), `rust/web/src/game/export.rs` (223), `rust/web/src/game/import.rs` (369) in the review snapshot. Cross-referenced: `web/src/nats.rs`, `web/src/db.rs` (`update_game_command_success`, `find_bot_turns`, `find_game_extended`), `web/src/main.rs`, `web/src/game/server_fns.rs` (submit_command), `web/src/email/commands.rs`, `web/src/email/sweep.rs`, `lib/game_client/src/lib.rs`.

## NATS handoff resolution (bot-command consumer ack/term/in_progress)

Evidence trace of `run_bot_command_consumer` (web/src/game/mod.rs:251-325):

1. **term(): NEVER called.** Grep for `\.term\(|nak\(|in_progress` across `web/src` returns zero matches. Permanently failing messages are handled by *leaving them unacked* (mod.rs:315-320). After `max_deliver = 3` (web/src/nats.rs:64,88-90) JetStream stops redelivering; the message strands in the WorkQueue stream with only a per-failure `tracing::warn` (mod.rs:316-319). There is no dead-letter subject, no advisory consumer, no alert. Confirmed problem (see findings below).

2. **Ack cadence: ack happens once, after ALL work completes, no `in_progress()` pings.** Sequence per message: pull (mod.rs:265) → parse (mod.rs:273) → `handle_bot_command_event` (mod.rs:284) which runs `execute_command` (game-service HTTP call with up to 3 retries/10s timeouts, lib/game_client/src/lib.rs:30 + web/src/main.rs:32-36, plus a multi-statement DB transaction in `update_game_command_success`, db.rs:1713-1781) and, on success, `notify_game_emails` (mod.rs:362-369 — sequential per-player email sends). Only then `message.ack()` (mod.rs:300/311). Realistic worst case is tens of seconds — well under the 5-min `ack_wait` — so duplicate delivery from ack_wait expiry is unlikely but not impossible (DB stall + resend slowness); there is no `in_progress()` extension as insurance.

3. **Error-path behaviour, per outcome:**
   - Unparseable payload: error log + **ack** (mod.rs:273-281). Correct (poison message dropped deliberately, via ack not term).
   - `Ok(())` or `Conflict`: ack (mod.rs:299-303). Conflict is resolved inside `handle_bot_command_event` by re-publishing `bot.turn` (mod.rs:390-397) or, at `attempt >= MAX_TURN_ATTEMPTS` (3), giving up and returning Ok (mod.rs:372-383) — i.e. **turn-level retry exhaustion is acked away; the bot turn is lost** and nothing re-drives it.
   - `UserError`: warn + ack (mod.rs:304-314). A bot command the game rejected is never retried AND `bot.turn` is not re-published — **the bot is still on turn and nothing ever re-triggers it** (see wedge finding).
   - `Other`: left unacked (mod.rs:315-320) → redelivered up to 3×, then **strands silently** (no term, no DLQ).
   - Ack failure itself is only warn-logged (mod.rs:300-302, 311-313, 277-279) → JetStream redelivers → duplicate processing.

4. **Duplicate delivery / idempotency:** there is no dedup on `BotCommandEvent.attempt` or any event id; idempotency rests entirely on `execute_command` re-reading fresh state (mod.rs:89) and the optimistic-concurrency `updated_at` guard in `update_game_command_success` (db.rs:1715-1728, `WHERE id = $4 AND updated_at = $5`, 0 rows → `StaleStateConflict`). A duplicate delivery of an already-applied command fails the `is_turn` check (mod.rs:103-105) → `Other("Not your turn")` → left unacked → burns all 3 deliveries → strands. The turn itself is safe, but each duplicate produces 3 noisy processing cycles plus a stranded message.

5. **No infinite loop:** every path either acks or hits the max_deliver=3 ceiling; `MAX_TURN_ATTEMPTS` bounds turn-level re-publishes (mod.rs:373). No unbounded retry found.

## Findings

### Bot command permanently rejected (UserError) wedges the game with the bot on turn
- severity: major
- category: correctness
- location: web/src/game/mod.rs:304-314 (consumer ack) + web/src/game/mod.rs:402-410 (handler)
- finding: When the game service rejects a bot's command as a user error (buggy bot, or bot computed against subtly different state), the consumer acks the message and nothing re-publishes `bot.turn`. The bot is still `is_turn = true` in the DB; the turn-reminder sweep only targets human users (web/src/email/sweep.rs:34-91), and no other path calls `trigger_bot_turns` for that game (it only runs as the epilogue of a successful `execute_command`, mod.rs:168 — which no human can perform while it's the bot's turn). The game is permanently stuck. Re-publishing `bot.turn` (bounded by the attempt counter) would let the bot recompute a fresh command from current state, which may well succeed where the stale/buggy one did not.
- recommendation: On `UserError` from a bot command, re-publish `bot.turn` with `attempt + 1` (same bounded path as Conflict) instead of acking into the void; only give up (ack) after `MAX_TURN_ATTEMPTS`, and emit a distinct error/metric for the wedged game.

### Turn-retry exhaustion silently abandons the bot's turn
- severity: major
- category: correctness
- location: web/src/game/mod.rs:372-383
- finding: After `MAX_TURN_ATTEMPTS` stale-state conflicts, `handle_bot_command_event` logs an error and returns `Ok(())`, which the consumer acks. Same wedge as above: bot still on turn, no re-drive mechanism, no alertable signal beyond a log line. Conflicts should be vanishingly rare (they need a concurrent write between the bot's read and commit), so exhaustion implies something is systematically wrong — exactly the case that deserves a loud, durable signal.
- recommendation: Treat exhaustion as a real failure: emit a metric/Sentry event, or park the message (term + DLQ subject) so stuck games are discoverable; consider a periodic sweeper that re-publishes `bot.turn` for games where a bot has been on turn longer than a threshold.

### Failed `bot.turn` publish after DB commit loses the bot turn
- severity: major
- category: correctness
- location: web/src/game/mod.rs:227-242 (publish failures warn-only), web/src/game/mod.rs:390-397 (conflict re-publish query failure warn-only), web/src/game/mod.rs:182-185
- finding: `publish_bot_turns` awaits the JetStream persistence ack (good), but on failure only logs a warn. Since the preceding `update_game_command_success` has already committed, the game now sits with a bot on turn and no event in the stream — permanently wedged (same absence of recovery as above). Same for the conflict path: if `find_bot_turns` fails, the re-publish is skipped silently and the original `bot.command` is then acked (mod.rs:400).
- recommendation: Same direction as the wedge findings: a reconciliation sweep ("bot on turn for > N minutes → re-publish bot.turn") fixes all three loss modes at once; short of that, surface publish failures as Err so the `bot.command` message stays unacked and redelivery retries the publish.

### Permanently failing bot.command messages strand in the stream after max_deliver (no term/DLQ)
- severity: minor
- category: correctness
- location: web/src/game/mod.rs:315-320 (leave-unacked path); consumer config web/src/nats.rs:63-94
- finding: Confirmed per handoff: `term()` is never called anywhere in `web/src`. A message that fails with `Other` on all 3 deliveries (e.g. game service down longer than the redelivery window, or the duplicate-delivery "Not your turn" case) just stops being delivered; it sits in the WorkQueue stream forever with no advisory handling, no metric, and no way to enumerate stranded messages operationally.
- recommendation: On `Other`, track deliveries (or use `message.info()` num_delivered) and `term()` at the ceiling with an error log + metric; or add a DLQ subject. At minimum, count/metric the stranded case.

### bot.command consumer is spawned once and never restarted if it exits
- severity: major
- category: correctness
- location: web/src/main.rs:55-74 + web/src/game/mod.rs:263-324
- finding: `run_bot_command_consumer` returns `Ok(())` when the `messages()` stream ends (mod.rs:322-324) and `Err` on setup/stream failures; the spawn site (main.rs:62-73) only logs the error — no supervisor loop, no retry. If the consumer's message stream ever terminates (consumer deleted/recreated by another replica's `ensure_stream_and_consumers` racing at startup, NATS-side error, etc.) every replica that experiences it silently stops driving bot turns until the pod restarts. UNCERTAIN: how often `consumer.messages()` terminates in practice depends on async-nats internals; the structural fragility (spawn-and-forget) is certain.
- recommendation: Wrap the consumer in a reconnect loop with backoff (the `Err` branch at main.rs:71 should restart, and the `Ok(())` stream-end path should also restart rather than exit).

### Stale `before` snapshot errors swallowed silently in handle_bot_command_event
- severity: nit
- category: quality
- location: web/src/game/mod.rs:344-347
- finding: `find_game_extended(...).await.ok().flatten()` discards a DB error without logging it. If the read fails, email notifications go out with `before = None` (notify_game_emails then treats `was_finished` as false and diffs against nothing), and the failure is invisible. Same pattern in submit_command (server_fns.rs:492-495) — consistent, but still silent.
- recommendation: Log a warn on the Err branch before falling back to `None`.

### Finished games wipe is_eliminated for previously eliminated players
- severity: minor
- category: correctness
- location: web/src/game/mod.rs:36-41 (status_fields Finished arm empties `eliminated`) + web/src/db.rs:1744 (`is_eliminated = status.eliminated.contains(&pos)`)
- finding: When a game transitions to Finished, `status_fields` emits `eliminated: vec![]` (the `Status::Finished` variant carries no eliminated list), and `update_game_command_success` unconditionally rewrites `is_eliminated` from it — so a player eliminated mid-game flips back to `is_eliminated = false` when the game finishes. Likely harmless today (finished games have `place` set and no turn reminders fire), but it silently rewrites historical per-player data. UNCERTAIN: nothing in the reviewed files reads `is_eliminated` for finished games; a UI/stats consumer elsewhere might.
- recommendation: Preserve the Active-arm eliminated list on finish (carry it in StatusUpdate), or make `update_game_command_success` not touch `is_eliminated` when `status.is_finished`.

### Conflict re-publish fans out to ALL bots currently on turn, not just the conflicting one
- severity: nit
- category: correctness
- location: web/src/game/mod.rs:390-392
- finding: On a stale-state conflict, `find_bot_turns` returns every bot on turn and all get a fresh `bot.turn` with `attempt + 1` — including bots whose turns are already in flight from an earlier publish (possible in simultaneous-turn games). Those extra publishes can produce duplicate `bot.command` events for bots that did not conflict; the duplicates are caught by the is_turn/updated_at guards, so the blast radius is noise + redelivery burn, not corruption. The shared attempt counter also advances for the bystander bots.
- recommendation: Re-publish only for the event's `player_position` (filtered from `find_bot_turns`), keeping the attempt semantics per-bot.

## Findings — export.rs

### Export bundle includes private log bodies despite "may get pasted into issues"
- severity: minor
- category: quality
- location: web/src/game/export.rs:1-4, 105-134
- finding: The module doc says the bundle "may get pasted into issues" and only excludes email addresses. But `game_logs` rows include private logs (`is_public = false`) with their full bodies (and the target positions), and the game_state blob itself may encode hidden information (other players' hands). An admin pasting a bundle into a public issue leaks in-game private communication/hidden state. UNCERTAIN: spec D4 may have accepted this deliberately — flagging for a conscious decision.
- recommendation: Either document in the module header that private logs/hidden state are included and bundles must not be posted publicly, or add a `--redact-private` mode to the export.

### BundlePlayer.bot_name actually holds game_bots.name (the seat display name), not the bot type
- severity: nit
- category: consistency
- location: web/src/game/export.rs:53-54, 142 vs web/src/game/import.rs:90-99
- finding: The field is named `bot_name` (matching `game_bots.bot_name`, the bot *type*, everywhere else in the schema) but is populated with `game_bots.name` (the per-game seat name) — the field comment documents this, and import.rs:83,93 correctly keys `bot_ids` by `bot.name`, so it round-trips correctly. Purely a naming trap for future readers/maintainers of the bundle format.
- recommendation: Rename the bundle field to `game_bot_name`/`seat_name` at the next schema_version bump, or leave as-is deliberately.

## Findings — import.rs

### Bundle timestamps (game.created_at/updated_at, log.created_at) captured on export but dropped on import
- severity: nit
- category: consistency
- location: web/src/game/export.rs:44-45,77 vs web/src/game/import.rs:59-68,140-150
- finding: `BundleGame.created_at`/`updated_at` and `BundleLog.created_at` are exported but never inserted (games insert omits created_at; game_logs insert only sets logged_at). Imported games all show "created just now", which skews any local list ordering by recency. Dev-only tool, so impact is cosmetic fidelity loss; also slightly odd that the schema carries fields the importer ignores.
- recommendation: Insert the bundle timestamps where columns allow, or drop them from the bundle to keep the schema honest.

### placeholder_user check-then-insert can race on username uniqueness
- severity: nit
- category: correctness
- location: web/src/game/import.rs:171-193
- finding: `taken` is checked, then `INSERT INTO users (name ...)` runs separately; a concurrent insert of the same name (or a validate/taken disagreement on case/Unicode edges) fails the whole import transaction with a unique-violation error rather than falling back to a generated name. Single-threaded dev CLI in practice, so low priority.
- recommendation: Catch the unique-violation on insert and retry with `generate_unique_username`, or rely on it unconditionally.

### Imported players get is_turn_at/last_turn_at = NOW() regardless of turn state
- severity: nit
- category: correctness
- location: web/src/game/import.rs:106-108
- finding: Both timestamp columns are set to NOW() for every imported player. For the player on turn this resets the turn-age clock (turn-reminder sweep eligibility, turn-duration stats); for others `last_turn_at` claims they just played. Dev-only; no user-visible harm beyond odd local stats/reminder timing.
- recommendation: Set `is_turn_at` = NOW() only when `player.is_turn`, and leave `last_turn_at` at the column default/NULL.

## Checked and found CLEAN

- **No panics/unwraps in request paths**: `mod.rs` (lines 1-422), `export.rs`, `import.rs` non-test code contain no `.unwrap()`/`.expect()`/`panic!`/`unreachable!`. The one fallible conversion (`HeaderValue::from_str`, export.rs:210-211) uses `unwrap_or_else` with a static fallback. All unwraps are inside `#[cfg(test)]` / `#[sqlx::test]`.
- **Transaction boundaries**: `update_game_command_success` (db.rs:1713-1781) wraps the games update, all per-player updates, rating changes, and log inserts in a single transaction — the concurrent-conflict test (mod.rs:659-735) verifies the failed transaction leaves no orphan log rows. `import_bundle` (import.rs:57-165) likewise runs entirely in one tx and propagates errors with `?`, rolling back cleanly.
- **Optimistic concurrency**: the `updated_at` compare-and-swap (db.rs:1715-1728) correctly closes the read-modify-write race in `execute_command`; the trigger-maintained `updated_at` does not interfere because the UPDATE sets it explicitly in the same statement.
- **Authorization**: `execute_command` itself takes a position (no user context) — appropriately, the callers enforce identity: `submit_command` maps the authenticated user to their seat via `game_players` (server_fns.rs:478-490, "You are not a player in this game"); the email path derives position from an authenticated token context (email/commands.rs:1194-1202); bot commands come only from the NATS stream. No path lets user A act on user B's seat. `admin_export_game` chains session lookup → token validation → `is_user_admin` before any data access (export.rs:182-202).
- **Expected rejections as data**: game `UserError` maps to `Ok(Some(msg))` in submit_command (server_fns.rs:519); Err is reserved for real failures. `ExecuteCommandError` cleanly separates Conflict/UserError/Other.
- **NATS publish durability**: `publish_bot_turns` awaits the JetStream persistence ack (mod.rs:227-238) — stronger than the "flush after publish" convention requires.
- **No infinite retry loops**: all retry budgets bounded (MAX_TURN_ATTEMPTS=3, max_deliver=3, game_client max_attempts=3).
- **Export auth/headers**: admin-gated, no emails in bundle, Content-Disposition filename built from a Uuid (no header-injection surface).
- **Bot-vs-human rating**: finished games with bot players skip rating changes; covered by tests (mod.rs:991-1064).
