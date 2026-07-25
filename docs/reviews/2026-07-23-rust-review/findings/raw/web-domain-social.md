# Raw findings: web domain - players.rs, friends.rs, new_game.rs

Scope: full review of rust/web/src/players.rs, rust/web/src/friends.rs, rust/web/src/new_game.rs in the 2026-07-23 snapshot, with targeted cross-referencing of web/src/db.rs (friends/blocks/search queries), web/src/game/server_fns.rs (restart authz), and migrations/010_friends.sql.

### Concurrent cross requests hit friends_pair_key instead of auto-accepting
- severity: minor
- category: correctness
- location: web/src/db.rs:1889 (surfaced via web/src/friends.rs:181)
- finding: `send_friend_request` does SELECT-then-INSERT inside a transaction with no row lock. If A->B and B->A are sent concurrently, both see `row: None` and both INSERT; one commit fails the `friends_pair_key` unique index (migrations/010_friends.sql:7) and the user gets a generic internal error instead of the intended mutual-intent auto-accept. Data stays consistent (the index holds), but the D1 "requester cannot distinguish outcomes" behavior is violated by an error on a legitimate request.
- recommendation: Catch the 23505 unique violation and retry the transaction once (the retry will take the `Some(r)` branch), or use `INSERT ... ON CONFLICT` on the pair expression.

### Friends page mutation errors are silently swallowed
- severity: minor
- category: quality
- location: web/src/friends.rs:426
- finding: Only `add_action` errors are rendered. `respond_action`, `unfriend_action`, `unblock_action`, `policy_action`, and `visibility_action` errors are never displayed anywhere: if Decline, Decline-and-block, Unfriend, Unblock, or a policy/visibility change fails server-side, the UI gives no feedback and (for the selects) keeps showing the value the user picked even though it was not saved.
- recommendation: Render `.value().get().and_then(|r| r.err())` for each action (a shared error slot is fine), and re-sync the selects from the refetched overview on failure.

### SetInvitePolicy success does not refetch the overview
- severity: minor
- category: consistency
- location: web/src/friends.rs:367
- finding: Every other mutation action has an `Effect` that bumps `set_refresh` on success (lines 373-398), including `visibility_action`, which mirrors the same select pattern. `policy_action` has no such effect, so after changing the invite policy the overview is stale; a subsequent unrelated refetch will re-render the select from stale-until-refetch data paths inconsistently with its sibling.
- recommendation: Add the same success `Effect` for `policy_action` as `visibility_action` has (or drop the refetch for both, since the selects are self-describing).

### block_user does not check the target exists
- severity: nit
- category: quality
- location: web/src/friends.rs:230
- finding: `block_user(user_id)` inserts straight into `blocks`; an unknown UUID trips the FK (`blocks ... REFERENCES users(id)`) and surfaces as a generic internal error, unlike `send_friend_request`, which resolves the target first and returns "User not found". Same for `unfriend`/`unblock_user` (harmless no-ops there, but blocked here it becomes a 500-class error).
- recommendation: Look up the user first (as `send_friend_request` does) or map the FK violation to a "User not found" error.

### get_friends_overview issues six sequential queries
- severity: nit
- category: quality
- location: web/src/friends.rs:99
- finding: friends, incoming, outgoing, blocked, invite_policy, and game_visibility are awaited one after another - six round trips on every friends-page load and refetch.
- recommendation: `tokio::try_join!` the six calls (they are independent reads on a pool), or fold invite_policy + game_visibility into one query.

### Restart prefill failure is silently swallowed
- severity: minor
- category: quality
- location: web/src/new_game.rs:271
- finding: `let Some(Some(Ok(pf))) = prefill.get() else { return; };` discards `Err` results from `get_restart_prefill` (game not found, not finished, "You are not a player in this game"). A user following a `?restart=<id>` link they are not entitled to, or with a stale id, gets a blank default setup form titled "Restarting X" with no indication the prefill failed.
- recommendation: Match the `Err` case and surface it via `set_form_error` (or render it near the heading), rather than proceeding as a normal new game.

### Email slots submitted unvalidated and untrimmed
- severity: minor
- category: correctness
- location: web/src/new_game.rs:374
- finding: `OpponentSlot::Email(email) => emails.push(email)` - Player slots are validated (line 367-372) but Email slots are pushed as-is: empty strings, whitespace, and untrimmed/uncanonicalized addresses all go to the server. This is the client end of W3's finding that invite emails are never trimmed/lowercased against the case-sensitive UNIQUE on user_emails; the form neither trims nor rejects obviously empty email slots.
- recommendation: Trim in `on_submit`, treat empty as a form error like the unselected-Player case, and lowercase to match whatever canonicalization the server adopts for W3's fix.

### Submit is a silent no-op when no version is selected
- severity: nit
- category: quality
- location: web/src/new_game.rs:355
- finding: `let Some(version_id) = selected_version_id.get_untracked() else { return; };` - with an empty `gt.versions` (or a version select parse failure at line 429 setting it to `None`), clicking Start game does nothing with no error, unlike every other validation path which sets `form_error`.
- recommendation: Set a form error ("No version available/selected") instead of returning silently.

### Create/restart outcome with no ids navigates nowhere
- severity: nit
- category: quality
- location: web/src/new_game.rs:316
- finding: The success effects only navigate when `outcome.game_id` or `outcome.proposal_id` is `Some`; a `ProposalOutcome` with both `None` (and `RestartOutcome::AlreadyRestarted { .. }` with both `None`, line 348) leaves the user on the form with the button re-enabled and no feedback, indistinguishable from nothing having happened even though the mutation succeeded.
- recommendation: Treat the both-None case as an error message, or make the server type make it unrepresentable.

### Bespoke percent-encoder instead of the percent-encoding crate
- severity: nit
- category: dependencies
- location: web/src/players.rs:35
- finding: `encode_path_segment` hand-rolls RFC 3986 unreserved-set percent-encoding (correctly, per its tests) and is used across players.rs, friends.rs, and new_game.rs. The `percent-encoding` crate (already in the dependency tree via url/reqwest ecosystems) provides exactly this with `utf8_percent_encode`.
- recommendation: Replace with `percent_encoding::utf8_percent_encode(s, NON_ALPHANUMERIC-adjusted set)` or keep the helper but delegate to the crate.

### Unbounded page number forwarded to the history server fn
- severity: nit
- category: correctness
- location: web/src/players.rs:771
- finding: `q.get("page").as_deref().unwrap_or("1").parse().unwrap_or(1)` then `.max(1)` accepts any i64 up to i64::MAX and forwards it to `get_player_history`; the offset multiplication overflow this enables lives server-side in stats (parallel to W4's client page offset overflow finding). Also `Some(d.page + 1)` at line 845 would overflow at i64::MAX, though `hide_next` normally suppresses that link.
- recommendation: Clamp page to a sane ceiling client-side and (per W4) saturate the offset math server-side.

### Restart prefill can select a player count not offered by the game type
- severity: nit
- category: correctness
- location: web/src/new_game.rs:274
- finding: `let count = (pf.opponents.len() + 1) as i32; ... set_player_count.set(count);` - if the game type's `player_counts` no longer includes the original game's count, no radio renders as checked (line 478 compares against the offered counts only) while the form still submits with the stale count; the server then re-validates, but the UI state is misleading.
- recommendation: Clamp the prefill count to the nearest offered count, or render the radios from the union including the prefill value.

## Checked and found CLEAN

- Authz: every server fn in friends.rs (`get_friends_overview`, `get_incoming_friend_request_count`, `send_friend_request`, `respond_to_friend_request`, `unfriend`, `block_user`, `unblock_user`, `set_invite_policy`, `set_game_visibility`, `get_opponent_suggestions`, `search_users`, `get_friend_activity`) calls `require_user` before touching the DB; all queries scope by the caller's id. `respond_to_friend_request` and `get_pending_request_source` both constrain on `target_user_id = responder`, so responding to someone else's request is impossible. `unfriend`/`unblock_user` can only affect rows involving the caller.
- new_game.rs server-fn surface: `get_available_game_types`, `get_available_bots`, `get_restart_prefill`, `restart_game_with_roster` all require auth; `get_restart_prefill_impl` checks is_finished and player membership (game/server_fns.rs:1149-1158). No unauthenticated path from this page.
- Silent-shield (D1/D7) semantics: target-blocked-source is a silent rollback inside `send_friend_request`; declined rows are indistinguishable from pending in `list_outgoing_friend_requests`; `unfriend` deletes only accepted rows so the declined shield survives; `block_user` atomically severs friends rows in a transaction. Decline-and-block's fetch-then-block pair is not transactional but is benign: `block_user` deletes the request row regardless.
- Input validation: `set_invite_policy`/`set_game_visibility` whitelist against the const tables before writing (plus DB CHECK constraints); `search_users` enforces min length, LIKE-wildcard escaping, self-exclusion, bidirectional block exclusion, and LIMIT 10 in db.rs:2442-2470; self-friend and self-block are rejected (friends.rs:170, 234) and backed by DB CHECKs.
- Query bounds: `friends_active_games`/`friends_recent_results` take LIMIT 10; opponent_suggestions bounds the recent tier to 20 games (friends tier is unbounded but proportional to the caller's own friend list); all friends-page lists are per-user scoped.
- No panics/unwraps in any request path across the three files; the only `unwrap_or`/`unwrap_or_default` uses have safe fallbacks. `win_pct` division at players.rs:644 cannot divide by zero (empty buckets are filtered at players.rs:180).
- players.rs pure helpers (`rating_trend`, `ordinal_suffix`, `format_placing`, `placing_histograms`, `game_types_for_profile`, `history_href`) are correct and well unit-tested, including 11th-13th ordinals, None-place exclusion, and non-ASCII path encoding. `opponents_view` correctly avoids nested anchors and renders bots as plain text.
- new_game.rs pure helpers (`player_range`, `filter_and_sort` with NaN-safe weight comparison, `prefill_to_slots`) correct and tested; slot resize preserves state on player-count changes; `taken` dedup prevents double-selecting a user across slots.
- XSS: all user-supplied strings render through Leptos text nodes (auto-escaped); URLs built via `encode_path_segment`.

Severity tally: 0 critical, 0 major, 5 minor, 7 nit.
