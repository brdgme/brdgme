# Raw findings — web-server-db-b (rust/web/src/db.rs lines 3313–6380)

Reviewer: Worker B (second half of db.rs). Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/web/src/db.rs`.
Scope confirmed: lines 3313–6380 (EOF). Prior worker covered 1–3312.

Findings appended incrementally as review progresses.

---

## Chunk 1: lines 3313–3812 (tests: friends/blocks/invite policy/game visibility)

(no findings yet — continuing)

---

## Consolidated findings (full pass complete over 3313–6380)

Note on scope: the entire assigned range (3313–6380) is inside the
`#[cfg(all(test, feature = "ssr"))] mod tests` block that begins at line 2961.
All non-test DB functions end by line 2959 (in worker A's range). So every
finding below concerns test code, test-enforced behavior, or coverage gaps in
the non-test code that these tests reveal. No SQL-injection/unwrap/panic
issues exist in my range that would violate project rules (unwraps in
`#[cfg(test)]` are permitted).

### Broad coverage gap: ~20 public DB functions have no test anywhere in the file
- severity: major
- category: quality
- location: web/src/db.rs:3313 (finding concerns the whole test module, 2961–6380)
- finding: Cross-referencing every `pub`/`pub(crate)` function defined in lines
  15–2959 against all test bodies, the following are never exercised by any
  test: `get_user_by_email` (166), `get_user` (184), `find_game` (323, only
  indirectly via `find_game_extended`), `find_game_version` (200),
  `find_latest_non_deprecated_game_version` (219), `find_game_version_rules`
  (258), `find_game_version_render_meta` (270), `find_enabled_bots` (537),
  `is_user_admin` (560), `find_user_id_by_name` (569),
  `find_open_restart_proposal`/`_tx` (792/806), `generate_unique_username`
  (856 — only the petname charset is tested at 3089, not the DB retry/conflict
  loop), `mark_game_read` (1394), `get_user_name`/`set_user_name` (2607/2620),
  `get_user_pref_colors`/`set_user_pref_colors` (2637/2648),
  `find_active_turn_games` (2923), `count_incoming_friend_requests` (2044),
  `should_hide_add_friend` (1998), `get_pending_request_source` (1951).
  Most are thin wrappers, but two have real logic worth covering:
  `find_active_turn_games` (ORDER BY is_turn_at ASC NULLS LAST + LIMIT, feeds
  the 22-day switch digest per its doc comment) and
  `generate_unique_username` (retry-on-conflict loop). Within otherwise very
  thorough tests, secondary behaviors are also unasserted:
  `find_active_game_summaries` test (4449) checks grouping/filtering but not
  ordering or any your-turn flag; `recent_games_for_index` tests (3859–3939)
  never create a game the user is NOT in, so the user-membership WHERE clause
  is untested; `find_finished_game_summaries` hard-3 cap is tested (4211) but
  only via exactly 4 games.
- recommendation: Add `#[sqlx::test]` coverage at minimum for
  `find_active_turn_games` (ordering, NULLS LAST, cap) and
  `generate_unique_username` (conflict retry), plus a foreign-game exclusion
  case for `recent_games_for_index`. The simple getters/setters
  (`get_user*`, `set_user_*`, `mark_game_read`, `is_user_admin`) can be
  covered cheaply in one round-trip test each.

### Test enshrines "un-finish" path that can leave is_finished=false with finished_at set
- severity: minor
- category: correctness
- location: web/src/db.rs:4685 (test `update_game_command_success_writes_finished_fields`; behavior under test is `update_game_command_success` at 1694, worker A's range)
- finding: The second `update_game_command_success` call passes
  `is_finished: false` for a game already finished. The test asserts
  `finished_at` is preserved via COALESCE (4706–4711), but does not assert
  what happens to `is_finished` — if the UPDATE writes `is_finished = false`
  (as the StatusUpdate implies), the row ends in the inconsistent state
  `is_finished = false AND finished_at IS NOT NULL`. The inline comment
  (4680–4684) itself flags the behavior as differing "from the plan's
  phrasing, see report" — i.e. the test author knew this was questionable and
  locked it in anyway. Uncertain: I did not re-verify the exact UPDATE column
  list in worker A's range; if `is_finished` is not written on this path the
  desync does not occur, but then the test documents nothing harmful and the
  comment is stale. Either way something needs reconciliation.
- recommendation: Worker A / lead should confirm whether
  `update_game_command_success` ever writes `is_finished = false` after a
  finish; if so, guard it (ignore non-finish updates on finished games, or
  clear finished_at). Also resolve the comment's dangling "see report"
  reference (no such report is linked in the repo).

### session_token_validation test documents absence of DB-side token expiry
- severity: minor
- category: correctness
- location: web/src/db.rs:5140 (test `session_token_validation`; subject is `crate::auth::session::validate_session_token`, outside db.rs)
- finding: The test deliberately asserts that a `user_auth_tokens` row with
  `created_at = NOW() - INTERVAL '40 days'` still validates (5146–5156):
  token expiry is enforced only by the tower_sessions cookie
  (`Expiry::OnInactivity(30 days)`), not by the DB existence check. If any
  caller ever treats `validate_session_token` as proof of a fresh session
  (e.g. a non-cookie bearer path), stale tokens are accepted. The test's NOTE
  comment documents this well, so it is a known/accepted design point, but it
  is security-relevant and worth the lead's attention when merging with the
  auth worker's findings.
- recommendation: Cross-check with the web-server-auth-crypto review; if
  cookie expiry is the only enforcement, consider a DB-side `created_at`
  window or a periodic token-cleanup job as defense in depth. No code change
  strictly required if the design is intentional.

### Test name over-promises: suggestions_exclude_blocked_and_self never tests self-exclusion
- severity: nit
- category: quality
- location: web/src/db.rs:4014
- finding: `suggestions_exclude_blocked_and_self` (4014–4032) covers both
  block directions well, but the "and_self" part is vacuous — a user can
  never be their own co-player via `make_game_with_players`, so the
  self-exclusion predicate in `opponent_suggestions` (2476) is not exercised.
- recommendation: Either rename the test or add a direct case (e.g. a
  corrupted/self game_players row, or unit-test the filter) if the
  self-exclusion clause is worth protecting.

### is_game_visible_to_user tests don't cover multi-player 'friends' tier or block interaction
- severity: nit
- category: quality
- location: web/src/db.rs:3584
- finding: `is_game_visible_to_user_friends_tier` (3584) and
  `_private_blocks_non_self` (3613) each use exactly one non-public player.
  The function's documented semantics (2224–2226) are "EVERY human player is
  public or (friends AND friend of viewer)"; the case of two 'friends'-tier
  players where the viewer is friends with only one (must NOT see) is
  untested, as is any interaction between blocks and game visibility.
- recommendation: Add a 3-player test: two 'friends'-visibility players,
  viewer friends with exactly one, assert not visible.

### count_rows test helper interpolates table name into SQL
- severity: nit
- category: consistency
- location: web/src/db.rs:6032
- finding: `format!("SELECT COUNT(*) FROM {}", table)` is string-built SQL.
  Test-only and only ever called with string literals (5823–5836), so there
  is no injection risk, but it is the kind of pattern that gets copy-pasted
  into non-test code.
- recommendation: Acceptable as-is; optionally hard-code the five counts or
  add a comment warning against reuse outside tests.

---

## Clean areas confirmed

- Lines 3313–4245 (friends/blocks/invite policy/visibility/index/dashboard/
  proposal-summary tests): assertions are specific and meaningful — exact
  message strings, ordering, both block directions, silent-shield semantics,
  email-of-existing-user policy path, bot-doesn't-block-ready edge case.
  `ALTER TABLE ... DISABLE TRIGGER` usage (3746, 3777, 3816, 3865, 3895,
  4222–4229) is safe because `#[sqlx::test]` provisions an isolated database
  per test; trigger state cannot leak between tests.
- Lines 4247–4520 (create_game/find_game_extended/bot-turns/summaries tests):
  good negative cases (missing ids return None/empty, not errors); the
  XOR user/bot constraint is verified both via the model and a raw query.
- Lines 4505–4944 (update_game_command_success undo-stash tests): thorough
  state-machine coverage — stash pinned to first command of a run, cleared on
  non-undoable command, cleared when opponent plays; eliminated-vs-placings
  type-confusion regression guard at 4561–4565 is a genuinely valuable test.
- Lines 4947–5117 (undo/concede/logs): log visibility (public vs targeted)
  verified per player; undo log line asserted.
- Lines 5165–5605 (ELO tests): zero-sum pairwise invariant, idempotent
  re-finish guard, bots excluded from rating, game_type_users row auto-create,
  rating_before capture — strong coverage. (Worker A already noted remaining
  choose_colors/ELO gaps; not re-flagged.)
- Lines 5786–5963 (delete_game/restart-link/search tests): cascade deletes
  verified by table counts, restarted_game_id NULLing, LIKE-wildcard escaping
  (`%`, `_`, `%%`) explicitly tested, blocked-user exclusion in both
  directions tested.
- Lines 6039–6380 (email management tests): primary-switch invariant,
  unverified/unknown rejection, global email uniqueness, verified-row
  immunity from expiry cleanup, inclusive 24h boundary — all well asserted.
- No panics/unwraps outside `#[cfg(test)]`, no string-built SQL with user
  input, no transaction-boundary or check-then-act issues originate in this
  range (it is entirely tests).

## Coverage confirmation

Read in full: lines 3313–6380 (EOF) of
`/home/beefsack/Development/brdgme-review-snapshot/rust/web/src/db.rs`,
in sequential chunks (3313–3812, 3812–4311, 4311–4860, 4860–5420,
5420–5960, 5960–6380), plus cross-reference reads at 2153–2232,
2894–2959, 2946–3312 and greps over the whole file and `migrations/`.
The range is 100% test module; every test in it was reviewed.
