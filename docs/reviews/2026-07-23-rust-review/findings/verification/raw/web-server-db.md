# Verification: web-server findings F34-F51 (web/src/db.rs)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5.
All line refs are snapshot lines. Verifier: independent read of cited regions
plus migrations 001/009/010/015 and the two undo call sites.

## F34 (major) undo_game does not clear rating state; re-finished game skips re-rating

- Verdict: CONFIRMED
- Evidence:
  - Idempotency guard, db.rs:1554-1557:
    `if players.iter().any(|p| p.rating_change.is_some()) { // Idempotency guard: this game has already been rated. return Ok(()); }`
  - `undo_game` (db.rs:1407-1463): the games UPDATE (:1417) resets
    `is_finished`/`finished_at`; the per-player UPDATE (:1438-1449) sets only
    `is_turn, is_eliminated, place, undo_game_state = NULL,
    turn_reminder_sent_at = NULL, updated_at` - it does NOT touch
    `rating_change`/`rating_before`, and nothing in the function touches
    `game_type_users`. Confirmed by full read of the function.
  - `delete_game` doc comment, db.rs:1342-1343: "Ratings are deliberately NOT
    rewound." - the no-rewind policy exists for admin deletes as claimed.
  - Consequence chain: a game that finishes (rated, `rating_change` stamped by
    :1665-1677), is undone, then finishes again re-enters
    `apply_rating_changes` via `update_game_command_success` :1775-1777 or
    `concede_game` :1332; the guard at :1554 sees the stale `rating_change`
    and returns without rating the new outcome. Confirmed.
- Reachability: stronger than the finding stated. The server undo path
  (game/server_fns.rs:731-784) checks only that `undo_game_state` is present -
  there is NO `is_finished` guard - and `update_game_command_success` stashes
  `undo_game_state` whenever `is_played && can_undo` (db.rs:1749-1753),
  including on the game-ending command. So the app layer permits undoing a
  finished game; the only gate is whether any engine emits `can_undo = true`
  on a game-ending command (external/out-of-scope, per the finding). Same
  shape at email/commands.cs:966 call site (email/commands.rs:966, verified
  present).
- Severity: major is right (correctness defect, silent wrong ratings;
  reachability engine-dependent keeps it below critical).
- Recommendation: valid. Clearing `rating_change`/`rating_before` in
  `undo_game` makes the next finish re-rate; note that without also rewinding
  `game_type_users` by the stored deltas the voided result's ELO remains
  applied AND the new result is rated on top (double-count of one game for
  the affected players). A correct fix should do both, or document the
  accepted skew. The finding's either/or phrasing ("rewind ... or recompute
  on next finish") slightly undersells this: "recompute on next finish"
  alone still leaves the voided deltas in `game_type_users`.

## F35 (major) ~20 public DB functions untested

- Verdict: ADJUSTED - core thrust confirmed, but the "worker A range"
  sentence is factually wrong about choose_colors and the ELO helpers.
- Evidence (grep of test module, lines >= 2961):
  - Zero test references: `find_active_turn_games`, `is_user_admin`,
    `get_user_by_email`, `mark_game_read`, `find_enabled_bots`,
    `set_user_name`, `get/set_user_pref_colors`,
    `find_open_restart_proposal`, `should_hide_add_friend` (all 0 hits).
    `generate_unique_username`'s single hit (:3090) is a comment, not a test.
    Spot-check confirms the coverage-gap thrust.
  - WRONG: "choose_colors ... the ELO helpers ... lack coverage".
    `choose_colors` has a dedicated table of tests at db.rs:5735-5780
    (choose_colors_honours_preference, ..._same_rank_conflict_resolves_distinctly,
    two legacy-normalization tests, ..._no_prefs_fills_from_palette_order).
    `elo_rating_change` is tested at :5167-5173 and :5175-5190
    (pairwise-sums-to-zero). Both are covered.
  - CORRECT: `concede_game` is tested (`concede_game_marks_finished`,
    :5021-5044) but only with 2 players; the 3+ player mis-placing
    behavior behind the `debug_assert!` at :1308 ("Only correct for 2-player
    games; callers must enforce") has no coverage and no release-build
    enforcement.
- Severity: major is defensible for a file docs/CODING.md requires tests for;
  keep major, but the finding text must drop the choose_colors/ELO claim.
- Recommendation: partially invalid - "choose_colors (table-driven #[test])"
  is already done; the rest (find_active_turn_games,
  generate_unique_username, concede_game 3+ hard check, getter round-trips)
  remains valid.

## F36 (minor) redundant updated_at = NOW() on trigger-maintained tables

- Verdict: ADJUSTED - true for 16 of the 18 listed lines; 2 listed lines are
  NOT redundant.
- Evidence:
  - migration 001_initial_schema.sql:25-32 defines `update_updated_at()`
    (`NEW.updated_at = now() AT TIME ZONE 'utc'`, unconditional) and
    :390-450 installs BEFORE UPDATE triggers on users, user_emails,
    user_auth_tokens, friends, chats, chat_users, chat_messages, game_types,
    game_type_users, game_versions, games, game_players, game_logs,
    game_log_targets. Manual `updated_at = NOW()` on those tables is dead
    (trigger overwrites it). Sampled lines 1293/1349/1417/1716 (games),
    1314/1396/1441 (game_players), 1654 (game_type_users), 1760
    (game_players), 1799/1937/2621/2649/2674/2688 (users), 1915 (friends) -
    all trigger-maintained. Confirmed.
  - WRONG entries: lines 1357 and 1363 update `game_proposals` (migration
    015), which has NO `update_updated_at` trigger - grep of all migrations
    shows only 001 installs it. Those two manual assignments are REQUIRED,
    not redundant. A blind sweep following the finding's line list would
    silently stop bumping game_proposals.updated_at.
  - Mixed idiom claim (NOW() vs timezone('utc', now())) confirmed (:1915 vs
    :1293).
- Severity: minor is right.
- Recommendation: FLAG - "sweep db.rs and drop manual updated_at
  assignments" is dangerous as written because the finding's own line list
  includes the two game_proposals lines that must be kept. The sweep must be
  scoped to migration-001 tables only (the finding's prose says this, but
  the location list contradicts it).

## F37 (minor) update_game_command_success can leave is_finished=false with finished_at set

- Verdict: CONFIRMED
- Evidence:
  - db.rs:1716: `UPDATE games SET game_state = $1, is_finished = $2,
    finished_at = COALESCE($3, finished_at), updated_at = NOW() WHERE id = $4
    AND updated_at = $5` - is_finished unconditional, finished_at coalesced,
    and :1710-1711 passes `finished_at = None` whenever
    `status.is_finished == false`. So a non-finish command on a finished game
    yields `is_finished = false AND finished_at IS NOT NULL`.
  - Test `update_game_command_success_writes_finished_fields` :4685-4711:
    drives exactly that second command (`is_finished: false` on a finished
    game) and asserts only `finished_at == Some(first_finished_at)`; there is
    no assertion on `is_finished` afterward. Confirmed.
  - Dangling comment confirmed at :4680-4684: "this differs from the plan's
    phrasing, see report" - the only "see report" in the file; no report
    reference resolvable from the code.
  - Reachability caveat stands: requires the game service to accept commands
    on finished games (not verified here).
- Severity: minor is right.
- Recommendation: valid.

## F38 (minor) apply_rating_changes zero-change results leave rating_change NULL

- Verdict: CONFIRMED
- Evidence:
  - Both write loops skip zero: :1647-1650 and :1666-1669
    (`if change == 0 { continue; }`), so a zero-change player gets no
    `rating_change` row update.
  - Exact tie between equally rated players: `elo_rating_change(r, r, 0.5)` =
    `round(K * (0.5 - 0.5))` = 0 (:1525-1528), so a drawn 2-player game
    between equal ratings computes change 0 for both, writes nothing, and the
    guard at :1554 stays unarmed - a second invocation re-runs (harmlessly,
    since it recomputes 0 again from unchanged ratings, but the
    "finished-and-rated => rating_change set" invariant is broken and any
    downstream "was this rated?" logic sees NULL).
  - Nuance worth recording: the guard is any-player, so partial-zero games
    (some players +/-N, one player net 0) DO arm the guard; only all-zero
    games are affected. The finding's example (exact tie, equal ratings) is
    the correct minimal case.
- Severity: minor is right (latent; double-run currently recomputes the same
  zeros, so no data corruption today).
- Recommendation: valid - writing rating_change = 0 and rating_before even
  for zero changes is safe and makes the guard total. (Adjust both loops or
  just the game_players loop; the game_type_users loop's skip is a pure
  no-op optimization and can stay.)

## F39 (minor) send_friend_request opposite-direction race to raw 23505

- Verdict: CONFIRMED
- Evidence:
  - db.rs:1889-1904: SELECT for either-direction row, then plain INSERT when
    None - separate statements in one READ COMMITTED tx, no ON CONFLICT, no
    lock.
  - migration 010_friends.sql:7-9: `CREATE UNIQUE INDEX friends_pair_key ON
    public.friends (LEAST(source_user_id, target_user_id),
    GREATEST(...))` - opposite-direction rows DO collide on this index, so
    the race loser gets a raw 23505 instead of the auto-accept at :1911-1921.
- Severity: minor is right (narrow window, wrong error surface, no
  corruption).
- Recommendation: valid; ON CONFLICT alone is subtle here because the
  conflict target is the expression index across BOTH directions - simplest
  correct fix is catching 23505 and re-running the read (auto-accept path).

## F40 (minor) friend_recent_visible_game N+1

- Verdict: CONFIRMED
- Evidence: db.rs:2322-2341 - candidate SELECT with LIMIT scan_limit, then
  `for ... { if is_game_visible_to_user(pool, game_id, viewer_id).await? }` -
  one query per candidate. The visibility predicate (:2233-2251) is a single
  pure-SQL EXISTS/NOT EXISTS and is mechanically inlinable.
- Severity: minor is right (bounded by scan_limit).
- Recommendation: valid, with one tradeoff to note: the code deliberately
  centralizes the predicate in `is_game_visible_to_user` ("shared ... so
  they cannot drift", doc comments near :2210, :2258). Inlining creates a
  second copy of the predicate and reintroduces drift risk the module
  currently guards against - an inlined version should carry a cross-ref
  comment or a shared SQL fragment constant.

## F41 (minor) insert_game_logs_tx row-at-a-time

- Verdict: CONFIRMED
- Evidence: db.rs:1238-1265 - per-log INSERT plus per-target INSERT,
  sequentially awaited inside the caller's tx.
- Severity: minor/nit boundary; minor acceptable. Volume is low (logs per
  command), tx already open.
- Recommendation: valid and appropriately hedged ("if profiling shows it
  matters ... otherwise leave").

## F42 (minor) db.rs 6.4k-line grab-bag

- Verdict: CONFIRMED
- Evidence: file is 6380 lines; test module starts :2961; section comments
  (e.g. `--- #30 friends ---` style markers observed around :3519, :4034,
  :5019, :5165, :5735) match the described organization.
- Severity: minor is right.
- Recommendation: valid (split-when-touched, not urgent).

## F43 (minor) build_game_type_user fabricates default rating row with nil-id sentinel

- Verdict: CONFIRMED
- Evidence: db.rs:99-108 - fallback arm returns `id: Uuid::nil()`,
  `user_id: default_user_id.unwrap_or(Uuid::nil())`, `rating: 1200,
  peak_rating: 1200`; no doc comment on the sentinel at the function
  (:59-71) or struct level.
- Severity: minor is right.
- Recommendation: valid.

## F44 (nit) update_game_command_success resets is_turn_at for continuing-turn players

- Verdict: CONFIRMED
- Evidence:
  - db.rs:1746: `let is_turn_at = if is_turn { now } else { p_is_turn_at };`
    - every player currently on turn gets `now`, including players who were
    already on turn.
  - migration 001:454-458: trigger `update_is_turn_at` fires only `WHEN
    (old.is_turn = false AND new.is_turn = true)` - so for a continuing-turn
    player the trigger does NOT fire and the manual `now` bind takes effect:
    the two mechanisms genuinely disagree, exactly as claimed.
- Severity: nit is right pending a semantics ruling; would rise to minor if
  is_turn_at feeds turn-age reminders (find_active_turn_games :2934 orders
  by it for the digest - a multi-action turn would look perpetually fresh).
- Recommendation: valid (confirm semantics first).

## F45 (nit) is_user_admin returns sqlx::Result vs anyhow neighbors

- Verdict: CONFIRMED
- Evidence: db.rs:560 `pub async fn is_user_admin(...) -> sqlx::Result<bool>`;
  grep shows the only other `sqlx::Result` uses in the file are two test fns
  (:2967, :2976) - it is the sole public non-test outlier.
- Severity: nit is right.
- Recommendation: valid.

## F46 (nit) generate_unique_username check-then-act race (mitigated)

- Verdict: CONFIRMED
- Evidence: db.rs:864-871 availability SELECT separate from the caller's
  later INSERT; migration 009_username_rules.sql:41 `CREATE UNIQUE INDEX
  users_name_lower_key ON public.users (lower(name))` backstops it - loser
  gets 23505 as described.
- Severity: nit is right.
- Recommendation: valid ("acceptable as-is").

## F47 (nit) delete_expired_unverified_emails interval via string round-trip

- Verdict: CONFIRMED
- Evidence: db.rs:2952-2955: `created_at < NOW() - ($1 || ' seconds')::interval`
  with `.bind(secs.to_string())`. Bound parameter - not injectable; just an
  int->text->interval round trip.
- Severity: nit is right.
- Recommendation: valid; `make_interval(secs => $1)` with an i64 bind works
  (make_interval's secs parameter is double precision - an explicit
  `$1::bigint` or letting Postgres coerce both work; recommendation fine).

## F48 (nit) send_friend_request no app-level self-request guard

- Verdict: CONFIRMED
- Evidence: db.rs:1877-1925 has no `source == target` check; migration
  001:114 `CONSTRAINT friends_check CHECK ((target_user_id <>
  source_user_id))` enforces it at DB level, surfacing as a generic error.
- Severity: nit is right.
- Recommendation: FLAG - a test at db.rs:3317 asserts
  `send_friend_request(&pool, a.id, a.id).await.is_err()`. The recommended
  silent `Ok(())` early-return inverts that contract and would break the
  existing test; implementing it requires deliberately updating the test,
  not just adding the guard. (Either contract is defensible; the current
  Err-on-self behavior is tested and deliberate.)

## F49 (nit) choose_colors clones prefs vec each pass

- Verdict: CONFIRMED
- Evidence: db.rs:970 `for (pos, pref) in rem_prefs.clone()` inside the
  `'outer` loop; the for-body (:971-980) mutates only `assigned` and
  `remaining`; `rem_prefs` is reassigned only after the for ends (:982-986).
  Iterating `&rem_prefs` borrows immutably with no conflict.
- Severity: nit is right (player counts tiny).
- Recommendation: valid (`for (pos, pref) in &rem_prefs` with `*pos`, or
  index iteration; borrow-checks cleanly).

## F50 (nit) apply_rating_changes convoluted all-pairs idiom

- Verdict: CONFIRMED
- Evidence: db.rs:1627-1632:
  `.iter().take(rated_players.len().saturating_sub(1)).enumerate()` outer +
  `.skip(a_index + 1)` inner - correct unordered-pairs enumeration, obscure
  phrasing. (The `take` is even redundant: skip(len) on the inner loop
  already yields nothing for the last element.)
- Severity: nit is right.
- Recommendation: valid; the suggested
  `for (i, a) in rated_players.iter().enumerate() { for b in
  &rated_players[i+1..] }` compiles (two immutable borrows) and is
  behavior-identical.

## F51 (nit) test-quality nits

- Verdict: ADJUSTED - parts (2) and (3) confirmed; part (1)'s reasoning is
  wrong and the sub-claim should be dropped or reworded.
- Evidence:
  - (1) REJECTED as stated: `suggestions_exclude_blocked_and_self` (:4014-4032)
    DOES exercise self-exclusion, just implicitly. The fixture makes `me` a
    game_player in the game (:4020-4028); the recent-co-players query
    (:2496-2511) would return `me` if the `op.user_id <> $1` filter
    (:2501) were removed, because the block-exclusion predicate never
    excludes self (no self-blocks exist). The final
    `assert!(...is_empty())` (:4031) would then fail - the test is
    mutation-sensitive to the self-filter. The finding's rationale ("a user
    can't be their own co-player via the fixture") is incorrect: the user IS
    their own co-player row in game_players. At most this is a
    "coverage-is-implicit, consider an explicit assertion" nit.
  - (2) CONFIRMED: `is_game_visible_to_user_friends_tier` (:3585-3611) has
    exactly one 'friends'-visibility player; no case with two 'friends'
    players where the viewer befriends only one (must NOT see, per the
    ALL-players predicate at :2235-2246). Missing.
  - (3) CONFIRMED: `count_rows` (:6032-6037) builds SQL with
    `format!("SELECT COUNT(*) FROM {}", table)` - test-only, called with
    literals; copy-paste-hazard characterization fair.
- Severity: nit is right.
- Recommendation: for (1), invalid as written (rename not needed - the name
  matches actual behavior); (2) and (3) recommendations valid.

## Summary of deviations from the prior review

- F35 ADJUSTED: choose_colors and elo_rating_change ARE tested
  (:5735-5780, :5167-5190); drop that sub-claim. Coverage-gap thrust and
  concede_game 3+ gap confirmed.
- F36 ADJUSTED: lines 1357/1363 target game_proposals, which has NO
  updated_at trigger - those manual sets are required. Sweep recommendation
  must exclude them.
- F51 ADJUSTED: part (1) rejected - the test does (implicitly) cover
  self-exclusion; parts (2)/(3) confirmed.
- F34 recommendation caveat: "recompute on next finish" without rewinding
  game_type_users double-counts; fix should clear player stamps AND rewind
  (or document the skew). Reachability is stronger than stated: no
  is_finished guard on the undo server path.
- F48 recommendation flag: existing test :3317 asserts Err on self-request;
  the silent-Ok recommendation breaks it unless the test is updated too.
- All other findings CONFIRMED at their stated severities; no severity
  changes recommended (F44 could rise to minor if is_turn_at is confirmed to
  drive turn-age digests - it feeds find_active_turn_games ordering :2934).
