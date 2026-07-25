# Raw findings: web/src/db.rs — part A (lines 1–~3200)

Reviewer: worker A. Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/web/src/db.rs`.
Coverage: lines 1–~3200 (boundary function finished in full; second worker owns onward).
All `sqlx::query!`/`query_as!` macro-vs-plain usage is per convention — not flagged.

## Findings

### concede_game sets updated_at manually despite trigger
- severity: nit
- category: consistency
- location: web/src/db.rs:1293
- finding: `UPDATE games SET is_finished = true, finished_at = NOW(), updated_at = NOW()` — `games.updated_at` is trigger-maintained (`update_games_updated_at` BEFORE UPDATE), so the manual `updated_at = NOW()` is dead weight; the trigger overwrites it. Harmless but misleading to readers (suggests the column must be set manually). Other UPDATEs in the file may or may not do the same — flagged once here as the pattern's first occurrence; later occurrences noted individually only if inconsistent.
- recommendation: Drop `updated_at = NOW()` from the SET list and rely on the trigger; sweep for the same pattern elsewhere in db.rs.

### insert_game_logs_tx is row-at-a-time (N+1-shaped writes)
- severity: minor
- category: quality
- location: web/src/db.rs:1238-1265
- finding: One `INSERT` per log plus one per log target, sequentially awaited inside the transaction. A command producing many logs/targets multiplies round trips while holding the transaction open. Volume is probably low (per-command logs), so low urgency.
- recommendation: If profiling shows it matters, use `QueryBuilder` to batch inserts (or `UNNEST` arrays). Otherwise leave; correctness is fine.

### generate_unique_username check-then-act race (mitigated)
- severity: nit
- category: correctness
- location: web/src/db.rs:864-871
- finding: The `SELECT true FROM users WHERE lower(name) = lower($1)` availability check and the caller's subsequent `INSERT INTO users` are separate statements; under READ COMMITTED a concurrent transaction could claim the same generated name between check and insert. Mitigated by the `users_name_lower_key` unique index (migration 009) — the loser gets 23505. For auto-generated petnames the failure surfaces as a game-creation error rather than a retry.
- recommendation: Acceptable as-is; optionally retry the whole create on 23505, or note the reliance on the unique index in the doc comment.

### choose_colors clones the whole prefs vec each outer-loop pass
- severity: nit
- category: quality
- location: web/src/db.rs:970
- finding: `for (pos, pref) in rem_prefs.clone()` clones the full `Vec<LocPref>` every iteration of `'outer` purely to appease the borrow checker (mutation is only to `assigned`/`remaining`, not `rem_prefs` — the clone is actually unnecessary since the loop body doesn't mutate `rem_prefs`). Player counts are small (< palette size), so impact is nil.
- recommendation: Iterate over `&rem_prefs` (no mutation occurs in the body) or iterate by index; drop the `.clone()`.

### is_user_admin returns sqlx::Result while neighbors use anyhow::Result
- severity: nit
- category: consistency
- location: web/src/db.rs:560
- finding: `pub async fn is_user_admin(...) -> sqlx::Result<bool>` is the only public DB fn in this range returning `sqlx::Result` instead of `anyhow::Result`; callers must juggle two error types. Doc comment says it was written to match `get_user_theme` (below), so the inconsistency may be deliberate/duplicated.
- recommendation: Unify on `Result<bool>` (anyhow) or document why sqlx::Result is intentional.

### build_game_type_user silently fabricates a default rating row
- severity: minor
- category: quality
- location: web/src/db.rs:59-110
- finding: On any NULL component of the LEFT JOINed `game_type_users` row, the function returns a synthetic `GameTypeUser` with `id: Uuid::nil()`, `rating: 1200`, `peak_rating: 1200` and `user_id` defaulting to `Uuid::nil()` when the joined user is also NULL (bot players). Callers downstream cannot distinguish "player has no rating row yet" from "real row with rating 1200" except by `id == Uuid::nil()`. This is presumably deliberate (new players start at 1200), but the sentinel is undocumented at the struct level and the partial-NULL case (some columns NULL, some not — impossible given the join, but not enforced) also collapses into the default.
- recommendation: Document the nil-id sentinel on `GameTypeUser` or the helper; consider `Option<GameTypeUser>` if any caller needs the distinction.

## Clean areas (lines 1–1298)
- `build_user_from_row` / `build_game_bot_from_row` LEFT-JOIN null handling with `ok_or_else` — correct, no unwraps.
- `find_game_extended` (400-506): join structure correct (`gtu` keyed on `u.id AND game_type_id`, bots get default gtu via sentinel), no N+1 (fixed 4 round trips), ordering `ORDER BY gp.position` preserved into Vec.
- `find_active_game_summaries` (583-645): ordering keys are per-game constant before `g.id`, so the contiguous-grouping loop in Rust is sound; `opp_id.is_some()` guard before `unwrap` of summary is correct (uses ok_or_else).
- `find_pending_game_summaries` / `find_finished_game_summaries` (661-770): grouping logic sound for same reason; subquery-with-ORDER-BY-LIMIT inside IN is valid PG.
- `create_game_with_users_tx` (1022-1214): proper single transaction for multi-statement writes; `ON CONFLICT DO NOTHING` for game_type_users; `fetch_optional + ok_or_else` for game_type_id; no unwraps.
- `validate_username` (842): byte-len vs char-count is equivalent here because the allowed charset is pure ASCII — correct.
- `choose_colors` recursion for overflow players mirrors documented intent (939-1004) — logic correct.


---

## Findings (lines ~1298–2695)

### Pervasive redundant `updated_at = NOW()` in UPDATEs — every touched table has a BEFORE UPDATE trigger
- severity: minor
- category: consistency
- location: web/src/db.rs:1293 (also 1314, 1349, 1357, 1363, 1396, 1417, 1441, 1654, 1716, 1760, 1799, 1915, 1937, 2621, 2649, 2674, 2688, and more below)
- finding: Migration 001 installs `update_updated_at` BEFORE UPDATE triggers on users, user_emails, friends, chats, chat_users, chat_messages, game_types, game_type_users, game_versions, games, game_players, game_logs, game_log_targets (migrations/001_initial_schema.sql:392-446) that unconditionally overwrite `NEW.updated_at`. Every `SET ... updated_at = NOW()` (or `timezone('utc', now())`) in this file is therefore dead SQL. This misleads readers into thinking manual maintenance is required (and inconsistent usage — some UPDATEs include it, some like `set_user_last_active` at 1814 deliberately omit it — makes the convention unclear). Note also the mixed idioms: `NOW()` vs `timezone('utc', now())` (friends fns at 1914/1937) — both overwritten anyway.
- recommendation: Sweep db.rs and drop all manual updated_at assignments on trigger-maintained tables; add a doc comment at the top of db.rs stating the trigger convention. (Supersedes the earlier nit at :1293.)

### send_friend_request: concurrent opposite-direction requests race to a 23505 error instead of auto-accept
- severity: minor
- category: correctness
- location: web/src/db.rs:1877-1925
- finding: The read-then-insert runs at READ COMMITTED. If A→B and B→A requests arrive concurrently and both read "no existing row", both attempt INSERT; the `friends_pair_key` unique index (migration 010) makes the loser fail with 23505, which propagates to the user as a raw error rather than being treated as mutual intent (auto-accept) per the function's own contract. Low probability (requires near-simultaneous mutual requests) but the failure mode is user-facing.
- recommendation: Map 23505 on the INSERT to a re-read + auto-accept of the reverse row, or use a single `INSERT ... ON CONFLICT` / advisory lock on the ordered pair.

### friend_recent_visible_game is N+1 by construction
- severity: minor
- category: quality
- location: web/src/db.rs:2316-2342
- finding: Fetches up to `scan_limit` candidate games, then issues one `is_game_visible_to_user` query per candidate until one passes. With a small scan_limit this is bounded, but the visibility predicate (`is_game_visible_to_user`, 2228-2252) is pure SQL and could be inlined into the candidate query, avoiding the per-row round trips and the drift risk of duplicating the predicate later.
- recommendation: Inline the NOT EXISTS visibility predicate into the candidate SELECT (single query), or add a `_tx`/SQL-fragment variant of the predicate usable inside a larger query.

### apply_rating_changes: outer-loop `take(len.saturating_sub(1))` is a convoluted "all pairs" idiom
- severity: nit
- category: simplicity
- location: web/src/db.rs:1627-1632
- finding: `rated_players.iter().take(len.saturating_sub(1)).enumerate()` paired with `.skip(a_index + 1)` computes each unordered pair once — correct, but the `take` only exists to skip the last (empty-inner-loop) element. `for (i, a) in rated_players.iter().enumerate() { for b in &rated_players[i+1..] }` says the same thing directly.
- recommendation: Rewrite with slice indexing as above; no behavior change.

### apply_rating_changes: zero-change results leave rating_change NULL, defeating the idempotency guard
- severity: minor
- category: correctness
- location: web/src/db.rs:1646-1677
- finding: The idempotency guard (1554) trips on "any player already has a rating_change", but the write loop skips players whose computed change is 0 (`if change == 0 { continue; }` at 1648/1667). In an exact-tie game between equally rated players (a_score 0.5 vs expected 0.5, K=32 → round(0)=0) EVERY change is 0, so no rating_change rows are written and a duplicate invocation would silently re-run (a no-op in that exact case, but the guard's invariant "finished-and-rated ⇒ rating_change set" is violated). Uncertain whether any caller can legitimately invoke this twice for one game; flagging as latent.
- recommendation: Write `rating_change = 0` (and rating_before) for rated players even when the change is 0, so the guard is reliable.

### update_game_command_success resets is_turn_at to `now` for players already on turn (true→true)
- severity: nit (uncertain — may be intended)
- category: correctness
- location: web/src/db.rs:1746
- finding: `let is_turn_at = if is_turn { now } else { p_is_turn_at };` resets the turn-start timestamp on every command for players who remain on turn across consecutive commands (games with multi-action turns). The `update_is_turn_at` trigger (migration 001:454) only stamps false→true transitions, implying the intended semantic is "when the turn started", and this code path partially duplicates/fights it. If is_turn_at drives "how long has it been their turn" UI or reminders, continuing-turn players look like their turn just started.
- recommendation: Confirm intended semantics; if "turn started", only set `now` when transitioning (`!p_was_turn && is_turn`) and let the trigger cover it; if "last activity", rename/document.

## Clean areas (lines 1298–2695)
- `delete_game` (1345-1390): correct FK-ordered deletes in one transaction; nulls restart/proposal links; returns rows_affected-based existence. Sound.
- `undo_game` (1407-1463): single tx; `finished_at = NULL` works because the `update_finished_at` trigger only fires false→true. Correct interplay.
- `get_game_logs` targeted-private-lines predicate (1497-1499) is correct (public OR targeted).
- `update_game_command_success` optimistic-concurrency guard `WHERE id = $4 AND updated_at = $5` + rows_affected check (1716-1728) is the correct pattern; trigger-maintained updated_at makes the token self-refreshing; tx drop rolls back on the early error return.
- `elo_*` helpers + pairwise multi-player application (1510-1644): math correct, bots excluded, ties score 0.5, additive `rating = rating + $1` avoids lost updates within the tx.
- `is_user_recently_active` fails open with an error log (1839-1858) — deliberate, documented. `active_within_window` conversion fallback is sane.
- Friends/blocks suite (1877-2148): silent no-op semantics per D1/D7 consistently implemented; `block_user` severs friends atomically in one tx; `respond_to_friend_request` uses rows_affected for exactly-once response; `search_users` LIKE escaping (`\`, `%`, `_` with `ESCAPE '\'`) is correct — no injection.
- `is_game_visible_to_user` / `is_game_publicly_visible` / `find_public_index_game_id` (2212-2287): predicates match their doc comments; bots correctly excluded via inner JOIN.
- `set_user_name` 23505 → Ok(false) mapping (2620-2631) matches the established convention.
- `opponent_suggestions`, `friends_active_games`, `friends_recent_results` (2476-2601): aggregates/grouping correct, block exclusions present, ORDER BY keys valid.


---

## Findings (lines ~2695–3312, end of worker-A range)

### delete_expired_unverified_emails builds an interval via string interpolation of a bound parameter
- severity: nit
- category: quality
- location: web/src/db.rs:2950-2957
- finding: `created_at < NOW() - ($1 || ' seconds')::interval` with `.bind(secs.to_string())` — not injectable (parameterized, and the value is an i64 formatted by Rust), but it round-trips an integer through text to build an interval. `NOW() - make_interval(secs => $1)` binds the i64 directly with no string conversion.
- recommendation: Use `make_interval(secs => $1::bigint)` (or `($1 * interval '1 second')` with a float8/numeric bind).

### send_friend_request has no application-level self-request guard
- severity: nit
- category: quality
- location: web/src/db.rs:1877
- finding: `send_friend_request(pool, a, a)` relies on the DB CHECK constraint (friends source<>target, asserted by the `self_request_rejected_by_db_check` test at 3314-3318, just past the boundary) to reject self-friending. The failure surfaces as a generic DB error rather than a domain outcome. Minor since callers presumably never pass self, and the DB does enforce it.
- recommendation: Early-return `Ok(())` (silent no-op, matching the function's other silent paths) when `source == target`, keeping the DB check as backstop.

## Test coverage observations (tests in range: 2961–3312)
- Tests in range cover: presence (`set_user_last_active`, `is_user_recently_active`, `active_within_window`), `find_available_game_types` weight/blurb, `validate_username`, petname charset, and the friends lifecycle (create/auto-accept/accept/decline/re-request/flip/pair-unique/self-check start). All `#[sqlx::test]` — they fail in plain local runs without a DB (known pre-existing condition, not flagged).
- NOT covered in this range (per docs/CODING.md "db.rs changes require tests" — flag as gaps, uncertain whether covered past the boundary):
  - `choose_colors` / `remove_highest_prefs` / `normalize_pref_color` — the color-assignment algorithm (940-1004) is the most intricate pure logic in the range and has no visible unit test in lines 1-3312. severity: minor. Recommend table-driven `#[test]` cases: tie-breaking, palette exhaustion → "Pink" fallback, tail recursion when players > palette, legacy name normalization.
  - `apply_rating_changes` — no test visible in range (searched for callers; only `concede_game`/`update_game_command_success` in this range). ELO math (`elo_rating_change`) is trivially unit-testable but untested here. severity: minor.
  - `concede_game`'s documented "only correct for 2-player games" constraint (1306-1308) is enforced only by `debug_assert!` — no test, and in release builds a 3-player game would silently assign place 1 to all non-conceders. Callers "must enforce" per the comment; consider a hard check returning an error. severity: minor (quality/robustness).

## Structure assessment (worker-A range only)
- Lines 1-3312 mix: row-builder helpers, game CRUD, game lifecycle writes (concede/undo/delete/update), ELO, presence, friends, blocks, user settings, multi-email management, visibility predicates, color assignment, plus ~350 lines of tests. Cohesion is poor — this is a grab-bag — but every function is individually small, documented, and consistently styled; section comments (`--- #30 friends ---`, `--- #22d multiple emails ---`) already mark natural module boundaries. severity: minor (simplicity). Recommendation: split along the existing section comments into `db/games.rs`, `db/friends.rs`, `db/users.rs`, `db/emails.rs` etc. when the file next needs major surgery; not urgent on its own.

## Clean areas (lines 2695–3312)
- `set_primary_email` (2845-2881): verify-then-swap in one transaction; partial unique index as backstop; enum outcome instead of bools. Sound.
- `remove_user_email` (2894-2917): the DELETE re-checks `is_primary = false` in its WHERE clause, so the read-then-delete race (address made primary between check and delete) cannot delete a primary. Good defensive pattern.
- `insert_unverified_email` 23505 → Ok(None) mapping (2802-2814) matches convention.
- Pure predicates (`can_remove_email`, `can_switch_to_email`, `is_expired_unverified`, `cap_digest`, 2732-2764) are pure, documented, and trivially testable — good factoring.
- `find_active_turn_games` (2923-2941): correct filter (is_turn AND not finished), sane ordering, cap bound as i64.
- All queries in range are parameterized — zero string-built SQL with user input anywhere in lines 1-3312. No `.unwrap()`/`.expect()`/`panic!` in any non-test DB function in range; Options are handled with `ok_or_else` or explicit match. Error swallowing: none found in range (`is_user_recently_active` fails open deliberately, with an error log, documented).


---

## Late finding (cross-check after range completion)

### undo_game does not clear rating_change / rewind game_type_users — a re-finished game keeps the annulled result's ratings (UNCERTAIN)
- severity: major (uncertain — depends on whether a finished game can be undone; see below)
- category: correctness
- location: web/src/db.rs:1407-1463 (undo_game) interacting with 1536-1680 (apply_rating_changes)
- finding: `apply_rating_changes` runs when a game finishes with placings, mutates `game_type_users.rating`/`peak_rating`, and stamps `game_players.rating_change` as its idempotency token. `undo_game` resets `is_finished`/`finished_at`/placings but does NOT clear `rating_change`/`rating_before` and does NOT rewind `game_type_users`. If the same game later finishes again (possibly with different placings), the guard at db.rs:1554 sees the old `rating_change` and skips re-rating entirely: players keep ELO computed from the voided first result, and the final result is never rated. Whether a finished game can actually be undone depends on whether game-ending commands are issued with `can_undo = true` (game-engine dependent; undo is reachable via `game/server_fns.rs:784` and `email/commands.rs:966`). `delete_game` documents "Ratings are deliberately NOT rewound" for admin deletes, so a no-rewind policy exists — but silently NOT rating the *new* outcome looks unintended rather than a policy. Marked uncertain; needs a product/engine ruling on finish-then-undo reachability.
- recommendation: If finished games can be undone: in `undo_game`, clear `rating_change`/`rating_before` and either rewind `game_type_users` by the stored deltas or recompute on next finish. If they cannot, add a comment in `undo_game` stating why the rating fields are left alone.

## Coverage confirmation
- Reviewed IN FULL: lines 1–3312 of `/home/beefsack/Development/brdgme-review-snapshot/rust/web/src/db.rs`, including the test module head (2961–3312). The function straddling the 3200 boundary (`pair_unique_index_rejects_reverse_duplicate`, 3293–3312) was finished; worker B owns from ~3200/3313 onward, including `self_request_rejected_by_db_check` (3314) and everything after.
- Cross-references consulted: migrations/001_initial_schema.sql (triggers 25-59, 392-464), migrations/010_friends.sql (unique indexes), migrations dir listing, callers in src/game/server_fns.rs and src/email/commands.rs.
- No panics/unwraps in non-test DB functions in range; no string-built SQL with user input in range; all multi-statement writes in range are transactional.
