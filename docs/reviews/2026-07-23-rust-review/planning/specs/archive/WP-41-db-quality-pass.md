# WP-41: `rust/web/src/db.rs` quality pass

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Close the db.rs quality backlog from the 2026-07-23 review: establish the `updated_at` trigger convention in writing and strip the 25 dead manual `updated_at = NOW()` assignments that the trigger already covers (ws F36), stop `update_game_command_success` from being able to leave `is_finished = false` alongside a non-NULL `finished_at` (ws F37), serialize `send_friend_request`'s read-then-insert so opposite-direction requests auto-accept instead of surfacing a raw 23505 (ws F39) and give it an application-level self-request no-op (ws F48), collapse `friend_recent_visible_game`'s per-candidate visibility round trips into one query with a drift-guard test (ws F40), unify `is_user_admin` on `anyhow::Result` (ws F45), replace the text-interpolated interval in `delete_expired_unverified_emails` with `make_interval` (ws F47), drop `choose_colors`' per-pass vec clone (ws F49), de-obfuscate `apply_rating_changes`' all-pairs loop (ws F50), document three deliberate-but-undocumented behaviours (`build_game_type_user`'s nil-id sentinel ws F43, `is_turn_at`'s last-activity semantics ws F44, `generate_unique_username`'s reliance on the unique index ws F46), fix the two surviving test-quality nits (ws F51), and close the headline coverage gap by adding 11 `#[sqlx::test]`s that cover 24 of the 27 currently-untested public DB functions — 25 of 27 once Task 2's `is_user_admin` test is counted (ws F35, the package's single major).

**Architecture — how `db.rs` works (read this before editing):**

`/home/beefsack/Development/brdgme/rust/web/src/db.rs` is **6877 lines**: production code at lines 1-3138, then `#[cfg(all(test, feature = "ssr"))]` at **:3139** and `mod tests {` at **:3140**, running to the file's final `}` at **:6877** (3739 lines, ~200 tests). Every production item is individually gated with `#[cfg(feature = "ssr")]` (there is no module-level gate). **Exactly one item is ungated: `validate_username` (:849)** — its own doc comment says it is shared with the client-side form. The other "pure predicate" helpers ARE `ssr`-gated despite being pure: `active_within_window` (cfg at :2001, fn :2002), `can_remove_email` (cfg :2909, fn :2910), `can_switch_to_email` (cfg :2916, fn :2917), `is_expired_unverified` (cfg :2923, fn :2924), `cap_digest` (cfg :2938, fn :2939). Do not "fix" that. **When you add or edit a production function, preserve its existing `#[cfg(feature = "ssr")]` attribute exactly.**

Section layout (production half), with the live line of each function you will touch:

- **Row builders** (:15-159): `build_user_from_row` :15, `build_game_bot_from_row` :38, `build_game_type_user` :59, `build_game_player_from_row` :115.
- **Lookups / getters** (:160-848): `create_pool` :160, `get_user_by_email` :170, `get_user` :188, `find_game_version` :204, `find_latest_non_deprecated_game_version` :223, `find_game_type_player_counts` :244, `find_game_version_rules` :262, `find_game_version_render_meta` :274, `find_available_game_types` :286, `find_game` :327, `find_game_extended` :404, `find_bot_turns` :527, `find_enabled_bots` :544, `is_player_in_game` :552, `is_user_admin` :567, `find_user_id_by_name` :576, the three `find_*_game_summaries` :590/:668/:729, `find_predecessor_game_id` :784, `find_open_restart_proposal_tx` :799, `find_open_restart_proposal` :813.
- **Username / colour helpers** (:849-1013): `validate_username` :849, `generate_unique_username` :863, `normalize_pref_color` :~900, `remove_highest_prefs` :911, `choose_colors` :947.
- **Game lifecycle writes** (:1014-1598): `create_game_with_users` :1014, `create_game_with_users_tx` :1029, `insert_game_logs_tx` :1227, `create_game_logs` :1278, `concede_game` :1291, `pick_replacement_bot` :1346, `replacement_bot_available` :1371, `concede_game_replace` :1382, `end_game` :1423, `delete_game` :1475, `mark_game_read` :1524, `undo_game` :1537.
- **Logs + ELO** (:1599-1868): `get_all_game_logs` :1599, `get_game_logs` :1619, `elo_transformed_rating` :1646, `elo_expected_score` :1651, `elo_rating_change` :1658, `write_ranked_placings` :1669, `apply_rating_changes` :1711.
- **The command write path** (:1860-1965): `StaleStateConflict` :1863, `update_game_command_success` :1869.
- **Theme / presence** (:1967-2037).
- **`// --- #30 friends ---` section** (:2038-2389): `FriendRow` :2042, `send_friend_request` :2055, `respond_to_friend_request` :2108, `get_pending_request_source` :2129, `unfriend` :2147, `are_friends_conn` :2161, `should_hide_add_friend` :2176, `list_friends` :2190, the request-list fns :2206-2253, `block_user` :2254, `unblock_user` :2280, `has_block` :2290, `has_block_conn` :2301, `list_blocked` :2316, invite-policy + game-visibility settings :2331-2372.
- **Visibility predicates** (:2373-2551): `find_game_visibility_for_users_tx` :2373, `is_game_publicly_visible` :2390, `is_game_visible_to_user` :2406, `find_public_index_game_id` :2439, `find_recent_game_log_lines` :2473, `friend_recent_visible_game` :2494, `recent_games_for_index` :2523.
- **Invite policy + user search / settings** (:2552-2887).
- **`// --- #22d multiple emails per account ---` section** (:2888-3138): pure helpers :2910-2946, `list_user_emails` :2947, `find_email_owner` :2962, `insert_unverified_email` :2975, `mark_email_verified` :2998, `set_primary_email` :3023, `remove_user_email` :3072, `find_active_turn_games` :3101, `delete_expired_unverified_emails` :3124.

**Database-side machinery you must not fight** (all in `/home/beefsack/Development/brdgme/rust/web/migrations/`):

- `update_updated_at()` (`001_initial_schema.sql:25-32`) sets `NEW.updated_at = now() AT TIME ZONE 'utc'` **unconditionally** on BEFORE UPDATE. It is attached by 14 `CREATE OR REPLACE TRIGGER update_<table>_updated_at` statements at **001:392-446** (first `update_users_updated_at` at :392, last `update_game_log_targets_updated_at` at :444-446) to exactly these 14 tables: `users`, `user_emails`, `user_auth_tokens`, `friends`, `chats`, `chat_users`, `chat_messages`, `game_types`, `game_type_users`, `game_versions`, `games`, `game_players`, `game_logs`, `game_log_targets`. (Verified: `grep -n "CREATE OR REPLACE TRIGGER" migrations/001_initial_schema.sql`.)
- **No later migration adds any trigger at all** (verified: `grep -rn "CREATE TRIGGER\|CREATE OR REPLACE TRIGGER" migrations/ | grep -v 001_initial` is empty). `bots`/`llm_providers` (`013_bot_efficacy.sql:10,20`) and `game_proposals`/`game_proposal_players` (`015_game_proposals.sql:8,22`) have `updated_at` columns with **no trigger** — manual sets on those tables are load-bearing.
- Three **conditional** BEFORE UPDATE triggers follow the 14 unconditional ones. Their line numbers are easy to get wrong; these are verified:
  - `update_finished_at` (function 001:34-41, trigger **001:448-452**) fires only `WHEN old.is_finished = false AND new.is_finished = true`.
  - `update_is_turn_at` (function 001:43-50, trigger **001:454-458**) fires only `WHEN old.is_turn = false AND new.is_turn = true`.
  - `update_last_turn_at` (trigger **001:460-464**) fires only `WHEN old.is_turn = true AND new.is_turn = false` — relevant because `update_game_command_success` also writes `last_turn_at` explicitly; the interaction is pinned by `update_game_command_success_mid_turn_keeps_last_turn_at` (db.rs:4759-4807).
- `friends_pair_key` (`010_friends.sql:7-9`) is `UNIQUE (LEAST(source,target), GREATEST(source,target))` — one row per unordered pair, so A→B and B→A collide.
- `friends_check` (001:114) is `CHECK (target_user_id <> source_user_id)`.
- `users_name_lower_key` (`009_username_rules.sql:41`) is the case-insensitive username unique index.
- `game_players.is_turn_at` is `timestamp without time zone NOT NULL` (001:193).

**Tech Stack:** Rust 1.97.0 (`rust/rust-toolchain.toml`), `sqlx` with Postgres, `anyhow::Result` as the file-wide error alias (`use anyhow::Result` at db.rs:6), `time::PrimitiveDateTime` for all timestamps, `#[sqlx::test]` for DB tests. Postgres 18 in the test harness.

**Global Constraints:**

- Run all commands from `/home/beefsack/Development/brdgme/rust`. **Per-package only.** `web` is feature-gated: `cargo test -p web --features ssr`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`. NEVER a workspace-wide `cargo build`/`check`/`test` (AGENTS.md "Resource constraints": ~30 binaries link).
- **`docs/CODING.md` mandates that every change to `rust/web/src/db.rs` lands with tests.** Every task below therefore ends with at least one test, even the documentation-only ones (for those, the "test" is the unchanged existing suite plus an explicit statement of which existing test covers the behaviour being documented).
- **`.sqlx` offline cache.** Several tasks change the SQL text of `sqlx::query!` / `query_as!` / `query_scalar!` **macro** calls. Those strings are keys into `/home/beefsack/Development/brdgme/rust/web/.sqlx/` (130 files), and CI runs `(cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`. **After the last SQL-changing task and before the final commit** you MUST regenerate the cache against a live migrated database and commit the `.sqlx/` diff. Per `/home/beefsack/Development/brdgme/docs/DEV.md:82-95`, using a disposable scratch DB:

```bash
createdb -h localhost -U brdgme_user brdgme_sqlx_prepare
DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_sqlx_prepare \
  sqlx migrate run --source rust/web/migrations
cd /home/beefsack/Development/brdgme/rust/web && \
  DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_sqlx_prepare \
  cargo sqlx prepare -- --tests --features ssr --all-targets
dropdb -h localhost -U brdgme_user brdgme_sqlx_prepare
```

  Verify with `SQLX_OFFLINE=true cargo check -p web --features ssr` and then `(cd /home/beefsack/Development/brdgme/rust/web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`. **Do not hand-edit `.sqlx/*.json`.**
- **Migrations are immutable.** No task here needs a schema change; if you think one does, stop and escalate. Do not edit any file in `rust/web/migrations/`.
- Run the full gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` before the **final** commit (it provisions throwaway Postgres 18 on 15432 and NATS on 14222 and runs the whole CI sequence). DB-test failures in a bare local run *without* it are pre-existing (AGENTS.md; backlog #40) — not a regression.
- **Every existing test must keep passing unmodified, with exactly two deliberate exceptions**, both called out inline: `update_game_command_success_writes_finished_fields` (db.rs:4810-4891, edited in Task 3) and `self_request_rejected_by_db_check` (db.rs:3492-3496, edited in Task 7). Any other test that goes red means your change was wrong — do not "fix" the test.
- Line numbers below are **live-file** numbers (see Snapshot drift). Tasks that shift numbering say so; later tasks locate by symbol name, not by line.
- Do not reformat, reorder or "tidy" code you are not asked to change. `cargo fmt --all -- --check` must pass; if `fmt` wants to reflow a query string you shortened, let it.

**Snapshot drift: YES — the file drifted substantially. Use live numbers only.**

`diff -u /home/beefsack/Development/brdgme-review-snapshot/rust/web/src/db.rs /home/beefsack/Development/brdgme/rust/web/src/db.rs` returns **13 hunks, +502/-5 content lines** (net +497: snapshot 6380 lines → live 6877 lines; snapshot tree is `f8763a5`). The findings' line citations are snapshot numbers and are wrong by up to +500. Drifted regions (all from the #47 concede/end-game work, commits `ecfc17a`..`0243472`), by hunk:

1. `build_game_player_from_row` gained `ranked_placing` and `left_at` (hunks `@@ -130 @@`, `@@ -149 @@`; live :115-159).
2. `find_game_extended`'s SELECT gained `gp.ranked_placing`, `gp.left_at` (hunks `@@ -426 @@`, `@@ -490 @@`; live :404-526).
3. **New functions inserted at live :1345-1473** (hunk `@@ -1335,6 +1342,129 @@`): `pick_replacement_bot`, `replacement_bot_available`, `concede_game_replace`, `end_game`. These did not exist at review time; three of them carry `updated_at = NOW()` and are folded into the F36 sweep below. `concede_game` itself is **unchanged** by the drift — it only shifted by +7 lines.
4. `undo_game`'s `game_players` UPDATE gained the `left_at = CASE ...` clause (hunk `@@ -1438,7 +1568,10 @@`; live :1569-1575).
5. **New function `write_ranked_placings`** (hunk `@@ -1527,6 +1660,48 @@`; live :1669-1710).
6. `apply_rating_changes` now reads `ranked_placing` and places via `p.ranked_placing.or(p.place)` (hunks `@@ -1538 @@`, `@@ -1579 @@`, `@@ -1620 @@`; live :1711-1855).
7. `update_game_command_success`'s `game_players` UPDATE gained `left_at = CASE ...` (hunks `@@ -1757 @@`, `@@ -1773 @@`; live :1931-1938).
8. One test-module hunk, `@@ -4942,6 +5120,325 @@` (+325 test lines).

Mapping for the findings you will need (all right-hand values re-verified against live source): F36's snapshot `:1293` → live **:1300**; snapshot `:1357/:1363` (the game_proposals lines to EXCLUDE) → live **:1487/:1493**; F37's snapshot `:1716` → live **:1891**, its test at snapshot `:4685` → live **:4810-4891**; F38's snapshot `:1646-1677` → live `:1711-1855` (**not yours**); F39/F48's snapshot `:1877` → live **:2055**, the Err assertion at snapshot `:3317` → live **:3495**; F40's snapshot `:2316-2342` → live **:2493-2520**; F43's `:59-110` → live **:59-110** (unchanged; the `#[cfg]` above it is at :56); F44's snapshot `:1746` → live **:1921**; F45's `:560` → live **:567**; F46's snapshot `:864-871` → live **:876-883**; F47's snapshot `:2950-2957` → live **:3123-3137** (the interval string is on live **:3131**); F49's snapshot `:970` → live **:977**; F50's snapshot `:1627-1632` → live **:1802-1807**; F51's snapshot `:4014/:3584/:6032` → live **:4193/:3763/:6529**.

---

## Disposition table — every finding re-derived from live source

| F# | Claim | Verdict | What this spec does, and why |
|---|---|---|---|
| **F35** (major) | ~20 public DB fns have no test | **ADJUSTED (major stands)** | Re-enumerated from live source: **27** public fns in db.rs:15-3138 have **zero** references anywhere in the db.rs test module (:3140-6877) **or** in `rust/web/tests/{ssr_pages,nats_bot_eventing,websocket_hygiene}.rs`. (Reproduce with the loop at the end of Task 11 run over all `pub fn`/`pub async fn` names above :3139.) The finding's list was right in thrust, incomplete in fact (it missed `find_game`, the four `game_version` lookups, `has_block_conn`, `get_pending_request_source`, `count_incoming_friend_requests`, `replacement_bot_available`, `insert_game_logs_tx`, `create_game_with_users_tx`, `create_pool`). Per the triage note, `choose_colors` and the ELO helpers are excluded — they ARE tested (live `choose_colors_*` tests; `elo_*` via the rating tests). Task 2 covers 1 (`is_user_admin`) and Task 11 adds **11 `#[sqlx::test]`s covering 24 more, i.e. 25 of the 27**; the 2 exclusions (`create_pool`, `create_game_with_users_tx`) and the routed remainder are justified in Task 11's cut rule. |
| **F36** (minor) | Redundant `updated_at = NOW()` on trigger-maintained tables | **ADJUSTED (minor stands)** | Premise holds. Re-swept live source instead of trusting the finding's 18-line list (which is both stale and incomplete): **25** assignments on the 14 trigger-maintained tables, listed line-by-line in Task 1. **Excludes live :1487 and :1493** (`game_proposals`, migration 015, **no trigger** — those manual sets are REQUIRED; removing them is a real regression). Also adds the file-header trigger-convention comment the finding asked for. |
| **F37** (minor) | `is_finished = false` with `finished_at` set is reachable | **CONFIRMED, recommendation ADJUSTED** | Live :1891 writes `is_finished = $2` unconditionally while `finished_at = COALESCE($3, finished_at)`; the test at :4810-4891 drives exactly that state and asserts only `finished_at` (:4885-4889). Of the finding's two options ("ignore non-finish updates on finished games, or clear finished_at"), **clearing `finished_at` is rejected**: `undo_game` (:1547) is the deliberate un-finish path and it *does* set `finished_at = NULL`, so clearing on the command path would erase the finish timestamp of a legitimately finished game on any stray non-finish command. Task 3 makes finish **sticky** — `is_finished = ($2 OR is_finished)` — which is exactly the invariant `COALESCE($3, finished_at)` already encodes for the sibling column, adds the missing `is_finished` assertion, and deletes the dangling "see report" comment at :4858-4862 (verified: those five lines end with "this differs from the plan's phrasing, see report."). |
| **F39** (minor) | Opposite-direction concurrent requests hit raw 23505 | **CONFIRMED, recommendation ADJUSTED** | `friends_pair_key` is the LEAST/GREATEST expression index (`010_friends.sql:7-9`), so the loser of the read-then-insert at :2067-2082 (read :2067-2075, `match row {` :2076, INSERT :2078-2082) gets a raw DB error instead of the documented auto-accept. Of the finding's three options, **`INSERT ... ON CONFLICT` is rejected**: inferring a conflict target on a two-expression index requires the inference expressions to match the index verbatim, and `DO NOTHING` would silently skip the auto-accept that is the whole point of the reverse-row branch. Task 7 takes the **transaction-scoped advisory lock on the ordered pair** — one extra statement, no inference subtleties, and every existing branch keeps its exact behaviour. |
| **F40** (minor) | `friend_recent_visible_game` is N+1 | **CONFIRMED, recommendation ADJUSTED** | Live :2514-2518 issues one `is_game_visible_to_user` query per candidate (up to `scan_limit`, and `index.rs:52` passes 10 **per friend** inside a `for (friend_id, friend_name) in friends` loop, so the real amplification is 10×friends). Inlined per the recommendation, **but** verification's caveat is honoured two ways: (a) the inlined predicate is preserved **verbatim** from the query string at :2412-2424 with a cross-reference comment in both functions, and (b) the scan-window semantics are preserved exactly by keeping `LIMIT $2` in a derived table rather than filtering-then-limiting (a bare `WHERE ... LIMIT $2` would silently start returning games *older* than the previous scan window). A drift-guard test asserts the two implementations agree over a 4-case visibility matrix. |
| **F41** (minor) | `insert_game_logs_tx` is row-at-a-time | **SKIPPED-BY-DECISION (comment only)** | Premise confirmed at :1246-1272 (per-log `INSERT INTO game_logs` at :1248-1259, per-target `INSERT INTO game_log_targets` at :1264-1268): 1 + N + M sequential awaited INSERTs. But the finding's own recommendation is conditional — *"If profiling shows it matters, batch ...; otherwise leave"* — and verification did not strengthen it. There is no profiling, N is bounded by the log count of a single game command (single digits; the only caller passing a non-empty vec is `update_game_command_success` at :1957 and `create_game_logs` at :1284), and batching would replace three compile-checked `query!` macros with `UNNEST` inserts whose `.sqlx` entries would then need hand-verifying. Task 9 records the decision and its trigger condition as a code comment so the finding is discharged rather than forgotten. **Do not batch.** |
| **F42** (minor) | db.rs is a 6.4k-line grab-bag | **FENCED — deferred out of this package** | Premise confirmed (live: 6877 lines, 3739 of them tests). A mechanical split is **rejected here** on concrete grounds: db.rs is on the declared path list of **nine** other packages (WP-35, WP-40, WP-45, WP-47, WP-49, WP-50, WP-52, WP-53, WP-59 — verified against `planning/work-packages.md`), **six** of which are decision-blocked (WP-35 D-12/D-14, WP-40 D-3, WP-45 D-8, WP-47 D-6, WP-49 D-6, WP-50 D-9) and will land later, so a file split now guarantees textual conflicts in every one of them; and every production item carries its own `#[cfg(feature = "ssr")]` (there is no module gate), so a split has to re-derive gating and visibility (`pub(crate)` vs `pub`) for ~150 items plus re-home a 3739-line test module — exactly the class of change that goes silently wrong. The finding itself says "when the file next needs major surgery; not urgent on its own". Task 1 lands the **module map** comment (the navigability half of the value) and the deferral is routed in "Cross-package / newly discovered". |
| **F43** (minor) | `build_game_type_user` fabricates a default rating row | **CONFIRMED, recommendation narrowed to the doc half** | Live :99-108 returns `id: Uuid::nil()`, rating/peak 1200 on any NULL join component. The finding's optional `Option<GameTypeUser>` is **not done**: grepped every consumer — the only field any caller reads off the synthetic row is `rating` (`game/server_fns.rs:369`), **no caller anywhere checks `id` for nil** (`grep -rn "Uuid::nil\|is_nil"` over `rust/web/src` returns only db.rs:100 and :104), and the struct lives in `crate::models::game`, which is **WP-53's path**. Task 5 documents the sentinel where it is produced. |
| **F44** (nit) | `is_turn_at` reset for continuing-turn players | **CONFIRMED as a fact, code-change recommendation OVERTURNED** | Live :1921 `let is_turn_at = if is_turn { now } else { p_is_turn_at };` does overwrite on every command, and the trigger only stamps false→true (001:454-458). But the *same UPDATE* also resets `turn_reminder_sent_at = NULL` (:1934), and the only consumer that reads `is_turn_at` as an elapsed-time threshold is the reminder sweep, which additionally requires `turn_reminder_sent_at IS NULL` (`email/sweep.rs:60-68`). The two fields are reset **together, in one statement**, which makes "last turn activity" the coherent, intentional reading — not an accident of two mechanisms fighting. Changing it would make the reminder sweep nag a player who acted 30 seconds ago. Renaming the column needs a migration, which is disproportionate for a nit. Task 4 documents the semantics at the assignment and names both consumers. **Do not change the assignment.** |
| **F45** (nit) | `is_user_admin` returns `sqlx::Result` | **CONFIRMED** | Live :567 is the only non-test `sqlx::Result` in the file. Verified the change is caller-invisible: there are **20** call sites outside db.rs (`grep -rn "is_user_admin" web/src --include=*.rs`) — 18 do `.map_err(internal("..."))` (`game/server_fns.rs:285,1353,1372`; `admin.rs:640,665,700,729,748,767,790,815,834,853,879,914,942,962,985`) and `internal` is `pub fn internal<E: std::fmt::Display>` (`error.rs:7`), which `anyhow::Error` satisfies; 1 `match`es on `Err(e)` and `tracing::error!("{e}")` (`game/export.rs:195-202`); and 1 is `.await.unwrap()` inside admin.rs's own test module (`admin.rs:2201`, module starts :2177), which also compiles against `anyhow::Result` because `anyhow::Error: Debug`. **Zero files outside db.rs change.** Task 2 (not Task 4). |
| **F46** (nit) | `generate_unique_username` check-then-act race | **CONFIRMED, recommendation narrowed to the doc half** | Live :876-883 SELECTs availability; the INSERT is the caller's. The finding's optional "retry on 23505" is **rejected as infeasible at this layer**: all four callers run inside an open transaction (`auth/server.rs:439`, `game/import.rs:181`, `proposals.rs:902`, `db.rs:1073`), and in Postgres a 23505 aborts the whole transaction, so a retry would need `SAVEPOINT` plumbing in four unrelated modules for a nit whose worst case is one failed game creation. Task 5 documents the reliance on `users_name_lower_key`. |
| **F47** (nit) | Interval built by string interpolation of a bound param | **CONFIRMED — not injectable; fix is a typing cleanup** | Live :3131 is `WHERE verified_at IS NULL AND created_at < NOW() - ($1 || ' seconds')::interval` with `.bind(secs.to_string())` at :3133. **Not an injection risk**: `$1` is a bound parameter, and its value is `i64::to_string()` of `Duration::as_secs()`, so the only characters that can reach the concatenation are ASCII digits, and Postgres concatenates *after* parameter binding — a hostile value could at worst make the `::interval` cast fail, never escape the literal. It is a pure round-trip-an-integer-through-text wart. Task 6 uses `make_interval`. |
| **F48** (nit) | No application-level self-request guard | **CONFIRMED** | Live :2055 relies solely on `friends_check` (001:114). Verified the silent-Ok is safe for users: the server fn already rejects self-friending with a proper user error *before* reaching db (`friends.rs:170-172`, `"You cannot friend yourself"`), so the db-layer guard is defense-in-depth on an unreachable path and degrades no UX. Per the triage note this breaks the Err assertion at live **:3495**; Task 7 rewrites that test in the same commit. |
| **F49** (nit) | `choose_colors` clones the prefs vec each pass | **CONFIRMED** | Live :977 `for (pos, pref) in rem_prefs.clone() {`; the body (:978-988) mutates only `assigned` and `remaining`, and `remove_highest_prefs(&rem_prefs)` (:989) is called after the `for` ends and its result is assigned back to `rem_prefs` (:990), so switching to `&rem_prefs` is borrow-sound. Task 5. |
| **F50** (nit) | `apply_rating_changes` all-pairs loop idiom | **CONFIRMED** | Live :1802-1807. `.take(rated_players.len().saturating_sub(1)).enumerate()` + `.skip(a_index + 1)` is equivalent to the slice form: when `i == len - 1` the slice `[len..]` is empty, so the `take` is redundant. `rated_players` is a `Vec<RatedPlayer>` built by `.push(...)` (:1780-1784), so it is sliceable. Task 5. **This function is also WP-40's (ws F38) — see landing order.** |
| **F51** (nit) | Three test-quality nits | **ADJUSTED — part (1) rejected, (2) and (3) done** | (1) **OVERTURNED**: `suggestions_exclude_blocked_and_self` (live :4192-4210) calls `make_game_with_players(&pool, version, me.id, &[blocked_by_me.id, blocked_me.id], 0, &[0])` at :4198-4206, and that fixture routes `creator_id` into a `game_players` row, so `me` **is** a player of the fixture game. A broken self-exclusion in `opponent_suggestions` *would* make the final `assert!(...is_empty())` at :4209 fail. The name does not over-promise; **do not rename it**. (2) and (3) are done in Task 10. |

**Findings in the `## db` section that are NOT in this package:** ws F34 (`undo_game` rating state) and ws F38 (`apply_rating_changes` zero-change guard) → **WP-40**.

---

## Non-Goals (owned elsewhere — do NOT absorb)

- **ws F34 / ws F38 and all undo/concede semantics** — owned by **WP-40 (BLOCKED-ON-DECISION D-3)**. Do NOT clear `rating_change`/`rating_before` in `undo_game`, do NOT rewind `game_type_users`, do NOT add TOCTOU guards to `undo_game`/`concede_game`, and do NOT change `apply_rating_changes`' `if change == 0 { continue; }` skips (live :1823 in the `for p in &rated_players` loop at :1821, and :1842 in the `for p in &players` loop at :1840). WP-40 also owns extracting `concede_core`/`undo_core`. Your only edits inside those three functions are the mechanical `updated_at` removals in Task 1 and the loop-idiom rewrite in Task 5.
- **The `concede_game` 3+-player hard check.** ws F35's recommendation includes "a hard check or error in `concede_game` for 3+ player games" (today only `debug_assert!(players.len() == 2, "concede_game assumes exactly 2 players")` at live :1315, with the place-assignment loop at :1316-1329). That is a **behaviour change inside WP-40's function** and is routed there. Do not add it, and do not write a test that pins the current release-build behaviour as if it were intended.
- **`web/src/stats/`, `web/src/index.rs`, `web/src/friends.rs`, `web/src/players.rs`, `web/src/game_info/` query performance** — owned by **WP-52**. In particular: the N+1 *at the caller* of `friend_recent_visible_game` (`index.rs:52` loops over friends issuing one call each) is WP-52's; Task 8 fixes only the N+1 *inside* the db function.
- **`web/src/game/server_fns.rs`, `web/src/game/mod.rs`, `web/src/models/game.rs`, `web/src/settings.rs`** — owned by **WP-53** (and WP-47/WP-45 for the visibility/bot-slot wiring). Do not change `GameTypeUser` to `Option`, do not touch `models/`, and do not edit any server fn.
- **`web/src/admin.rs`** — owned by **WP-37**, including the `bot_providers`/`bots` manual `updated_at` sets, which are **required** (no trigger on migration-013 tables). Your F36 sweep must not leave db.rs and must not "helpfully" extend into admin.rs.
- **`web/src/proposals.rs` and `web/src/email/sweep.rs`** — owned by **WP-44/WP-46**. They contain four more `($1 || ' seconds')::interval` sites (`proposals.rs:725`, `proposals.rs:755`, `proposals.rs:819`, `email/sweep.rs:65` — verified with `grep -rn "seconds')::interval" /home/beefsack/Development/brdgme/rust/web/src`, which returns exactly those four plus db.rs:3131). Task 6 fixes the db.rs one only; the others are routed below. Do not touch those files.
- **`web/src/auth/server.rs`, `web/src/crypto.rs`** — WP-34/WP-35/WP-36.
- **Converting the #30 friends plain queries to `query!` macros** — that is `docs/BACKLOG.md` item 49, explicitly flagged "review first to ensure direction is correct before executing". Do not convert anything; keep plain queries plain and macro queries macros.
- **Splitting db.rs into modules (ws F42)** — deferred, see the disposition table and the routing note below.
- **Batching `insert_game_logs_tx` (ws F41)** — skipped by decision, comment only.
- **Any migration.** No `rust/web/migrations/` file is created or modified.

### Coordination / landing order

| Function | This package's edit | Other package also edits it | Order |
|---|---|---|---|
| `concede_game` (:1291) | Task 1 removes `updated_at = NOW()` at :1300 and :1321 | **WP-40** (TOCTOU guards, `concede_core` extraction, 3+-player check) | **WP-41 first.** WP-40 is BLOCKED-ON-DECISION D-3 and will restructure the function; rebasing two deleted clauses onto a restructure is trivial, the reverse is not. |
| `undo_game` (:1537) | Task 1 removes `updated_at = NOW()` at :1547 and :1574 | **WP-40** (rating rewind, guards) | **WP-41 first**, same reason. |
| `apply_rating_changes` (:1711) | Task 1 removes `updated_at = NOW()` at :1829; Task 5 rewrites the all-pairs loop header at :1802-1807 | **WP-40** (ws F38: write `rating_change = 0` instead of skipping) | **WP-41 first.** The two edits are in disjoint regions (the pair loop and the `updated_at` clause vs. the two `if change == 0 { continue; }` write loops at :1823 and :1842), so this is a clean textual merge either way, but landing the cosmetic change first keeps WP-40's diff readable. |
| `is_game_visible_to_user` (:2406) | Task 8 adds a cross-reference comment only (the SQL is unchanged) | **WP-47** (wire the predicate into game details + stats, D-6) | **WP-41 first.** WP-47 adds *callers*; Task 8 adds a second in-file copy of the predicate that WP-47 must be told about — hence the mandatory comment. |
| `friend_recent_visible_game` (:2494) | Task 8 rewrites the query | none | — |
| `is_user_admin` (:567) | Task 4 changes the return type | callers in **WP-37** (`admin.rs`) and **WP-53/WP-47** (`server_fns.rs`) | **WP-41 first** — verified caller-source-compatible, so those packages need no coordination, but landing after them would force a re-verify of any new call sites they add. |
| `mark_email_verified` (:2998), `set_primary_email` (:3023), `delete_expired_unverified_emails` (:3124) | Task 1 removes `updated_at = NOW()` at :3000, :3043, :3050; Task 6 rewrites `delete_expired_unverified_emails`' query | **WP-50** (email canonicalization, D-9) and **WP-59** (inbound processing) both declare `web/src/db.rs` | **WP-41 first.** All three edits here are one-clause/one-string changes; both other packages are about *which* address is written, not how `updated_at` is maintained. |
| `.sqlx/` cache | regenerated once, at the end | every db.rs-touching package | Whoever lands second regenerates again. Note it in the PR description. |
| `db.rs` module split (ws F42) | not done | would collide with all nine (incl. WP-59) | Deferred until the decision-blocked packages land. |

---

## Task 1: document the trigger convention, add the module map, and sweep the 25 dead `updated_at = NOW()` assignments (ws F36; ws F42 partial)

**Problem:** `update_updated_at()` (`migrations/001_initial_schema.sql:25-33`) unconditionally overwrites `NEW.updated_at` on BEFORE UPDATE for 14 tables. Every manual `updated_at = NOW()` in an UPDATE against one of those tables is dead SQL: it computes a value that the trigger immediately discards. It is applied inconsistently (some UPDATEs have it, some don't), mixes two idioms (`NOW()` and `timezone('utc', now())`), and — worst — teaches the next reader that manual maintenance is required, which is how the `game_proposals` sets at :1487/:1493 come to look removable when they are not.

**Why a header comment first:** the sweep is only safe if the rule is written down. Without it, the next person adding an UPDATE either re-adds dead SQL or omits a *required* set on `bots`/`game_proposals`.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs` only.

**Steps:**

- [ ] Insert this comment block at the very top of `db.rs`, **above** the existing `#[cfg(feature = "ssr")] use crate::game::StatusUpdate;` at line 1:

```rust
//! Database access layer.
//!
//! # `updated_at` convention
//!
//! `migrations/001_initial_schema.sql:25-32` defines `update_updated_at()` and
//! attaches it as a BEFORE UPDATE trigger (001:392-446) to these 14 tables:
//! `users`, `user_emails`, `user_auth_tokens`, `friends`, `chats`,
//! `chat_users`, `chat_messages`, `game_types`, `game_type_users`,
//! `game_versions`, `games`, `game_players`, `game_logs`, `game_log_targets`.
//! The trigger overwrites `NEW.updated_at` unconditionally, so **never write
//! `updated_at` by hand in an UPDATE against one of those tables** - the
//! assignment is dead SQL (ws F36).
//!
//! Tables added by later migrations have `updated_at` columns but **no
//! trigger**: `bots` and `llm_providers` (013_bot_efficacy.sql:10,20) and
//! `game_proposals` / `game_proposal_players` (015_game_proposals.sql:8,22).
//! Manual `updated_at` maintenance on those tables is REQUIRED - see
//! `delete_game`, which nulls two `game_proposals` FK columns and must keep
//! its manual sets.
//!
//! Three other BEFORE UPDATE triggers are conditional and are NOT substitutes
//! for an explicit write: `update_finished_at` fires only on `is_finished`
//! false -> true (001:448-452), `update_is_turn_at` only on `is_turn`
//! false -> true (001:454-458), and `update_last_turn_at` only on `is_turn`
//! true -> false (001:460-464).
//!
//! # Module map
//!
//! This file is deliberately one module (a split is tracked as review finding
//! ws F42, deferred while several decision-blocked work packages still have
//! pending edits here). Section order:
//!
//! - row builders (`build_*_from_row`, `build_game_type_user`)
//! - lookups and getters (`get_user*`, `find_game*`, `find_*_summaries`)
//! - username and colour helpers (`validate_username`,
//!   `generate_unique_username`, `choose_colors`)
//! - game lifecycle writes (`create_game_with_users*`, `concede_game*`,
//!   `end_game`, `delete_game`, `undo_game`)
//! - logs and ELO (`insert_game_logs_tx`, `elo_*`, `write_ranked_placings`,
//!   `apply_rating_changes`)
//! - the command write path (`update_game_command_success`)
//! - theme and presence
//! - `#30` friends and blocks
//! - game-visibility predicates (`is_game_visible_to_user` and friends)
//! - invite policy, user search, user settings
//! - `#22d` multiple emails per account
//! - `#[cfg(all(test, feature = "ssr"))] mod tests`
//!
//! Every production item is individually `#[cfg(feature = "ssr")]`-gated -
//! there is no module-level gate. The single exception is
//! `validate_username`, which is ungated so the client-side settings form and
//! the server fns share one definition. The other pure predicates
//! (`active_within_window`, `can_remove_email`, `can_switch_to_email`,
//! `is_expired_unverified`, `cap_digest`) are `ssr`-gated even though they are
//! pure; every caller is server-side, so leave them gated.
```

  **Do not paraphrase the trigger line numbers above; they were verified against `migrations/001_initial_schema.sql` and an earlier draft of this spec had them wrong.** If you want to re-check: `grep -n "CREATE OR REPLACE TRIGGER\|CREATE OR REPLACE FUNCTION public.update_" /home/beefsack/Development/brdgme/rust/web/migrations/001_initial_schema.sql`.

- [ ] **Remove `updated_at = NOW()` (or `updated_at = timezone('utc', now())`) from exactly these 25 sites.** Work top-down; line numbers shift as strings collapse, so locate each by its enclosing function name. Each entry gives the enclosing function and the resulting SQL verbatim.

  All 25 sites, and only these 25, are what `grep -n "updated_at = NOW()\|updated_at = timezone" db.rs | awk -F: '$1<3140'` returns **minus** the two `game_proposals` lines (:1487, :1493) — that grep returns 27 lines today. Line numbers below point at the exact `updated_at` line; the quoted "after" text is the whole enclosing string literal.

  1. `concede_game`, live :1300 → `"UPDATE games SET is_finished = true, finished_at = NOW() WHERE id = $1"`
  2. `concede_game`, `updated_at` on live :1321, literal :1319-1322 →
```rust
            r#"UPDATE game_players
               SET is_turn = false, place = $1, undo_game_state = NULL,
                   turn_reminder_sent_at = NULL
               WHERE id = $2"#,
```
  3. `concede_game_replace`, `updated_at` on live :1397, literal :1395-1398 →
```rust
        r#"UPDATE game_players
           SET is_turn = false, game_bot_id = $1, left_at = NOW(),
               undo_game_state = NULL, turn_reminder_sent_at = NULL
           WHERE id = $2"#,
```
  4. `end_game`, live :1427 → `"UPDATE games SET is_finished = true, finished_at = NOW() WHERE id = $1"`
  5. `end_game`, `updated_at` on live :1444, literal :1442-1445 →
```rust
            r#"UPDATE game_players
               SET place = $1, is_turn = false, undo_game_state = NULL,
                   turn_reminder_sent_at = NULL
               WHERE id = $2"#,
```
  6. `delete_game`, live :1479 → `"UPDATE games SET restarted_game_id = NULL WHERE restarted_game_id = $1"`
  7. `mark_game_read`, live :1526 → `"UPDATE game_players SET is_read = true WHERE game_id = $1 AND user_id = $2"`
  8. `undo_game`, live :1547 → `"UPDATE games SET game_state = $1, is_finished = $2, finished_at = NULL WHERE id = $3"`
  9. `undo_game`, `updated_at` on live :1574, literal :1569-1575 →
```rust
            r#"UPDATE game_players
               SET is_turn = $1, is_eliminated = $2, place = $3, undo_game_state = NULL,
                   turn_reminder_sent_at = NULL,
                   left_at = CASE WHEN is_eliminated = false AND $2 = true
                                  THEN NOW() ELSE left_at END
               WHERE id = $4"#,
```
  10. `apply_rating_changes`, `updated_at` on live :1829, literal :1827-1831 →
```rust
            r#"
            UPDATE game_type_users
            SET rating = rating + $1, peak_rating = GREATEST(peak_rating, rating + $1)
            WHERE game_type_id = $2 AND user_id = $3
            "#,
```
  11. `update_game_command_success`, live :1891 — **combined with Task 3; leave this one for Task 3** so the whole string changes once.
  12. `update_game_command_success`, `updated_at` on live :1937, literal :1931-1938 →
```rust
            r#"UPDATE game_players
               SET is_turn = $1, place = $2, is_eliminated = $3, points = $4,
                   undo_game_state = $5, last_turn_at = $6, is_turn_at = $7,
                   turn_reminder_sent_at = NULL,
                   left_at = CASE WHEN is_eliminated = false AND $3 = true
                                  THEN NOW() ELSE left_at END
               WHERE id = $8"#,
```
  13. `set_user_theme`, live :1977 → `sqlx::query("UPDATE users SET theme = $1 WHERE id = $2")`
  14. `send_friend_request`, `updated_at` on live :2093, literal :2092-2093 → `sqlx::query("UPDATE friends SET has_accepted = TRUE WHERE id = $1")` (this collapses a 2-line literal to 1 line; the surrounding `sqlx::query(` / `)` at :2091 / :2094 may be collapsed by `cargo fmt` too — let it)
  15. `respond_to_friend_request`, `updated_at` on live :2115, literal :2115-2116 →
```rust
        "UPDATE friends SET has_accepted = $1
         WHERE id = $2 AND target_user_id = $3 AND has_accepted IS NULL",
```
  16. `set_invite_policy`, live :2341 → `sqlx::query("UPDATE users SET invite_policy = $1 WHERE id = $2")`
  17. `set_game_visibility`, live :2362 → `sqlx::query("UPDATE users SET game_visibility = $1 WHERE id = $2")`
  18. `set_user_name`, live :2799 → `sqlx::query("UPDATE users SET name = $1 WHERE id = $2")`
  19. `set_user_pref_colors`, live :2827 → `sqlx::query("UPDATE users SET pref_colors = $1 WHERE id = $2")`
  20. `set_user_turn_emails_enabled`, live :2852 → `sqlx::query("UPDATE users SET turn_emails_enabled = $1 WHERE id = $2")`
  21. `set_user_invite_emails_enabled`, live :2866 → `sqlx::query("UPDATE users SET invite_emails_enabled = $1 WHERE id = $2")`
  22. `set_user_reminder_emails_enabled`, live :2880 → `sqlx::query("UPDATE users SET reminder_emails_enabled = $1 WHERE id = $2")`
  23. `mark_email_verified`, live :3000-3001 →
```rust
        "UPDATE user_emails SET verified_at = NOW()
         WHERE user_id = $1 AND email = $2 AND verified_at IS NULL",
```
  24. `set_primary_email`, live :3043-3044 →
```rust
        "UPDATE user_emails SET is_primary = false
         WHERE user_id = $1 AND is_primary = true",
```
  25. `set_primary_email`, live :3050-3051 →
```rust
        "UPDATE user_emails SET is_primary = true
         WHERE user_id = $1 AND email = $2",
```

- [ ] **DO NOT TOUCH** `delete_game`'s two `game_proposals` UPDATEs at live :1486-1488 and :1492-1494 (`updated_at` on :1487 and :1493). They target migration-015 tables which have **no** `update_updated_at` trigger; the manual sets are the only thing keeping `game_proposals.updated_at` accurate. There is already a two-line explanatory comment at :1484-1485 — replace those two lines with this four-line block (the first two lines are the existing text verbatim):

```rust
    // game_proposals (migration 015) FK-reference games via started_game_id and
    // restarted_game_id; null both or the game delete violates those FKs.
    // NOTE: game_proposals has NO update_updated_at trigger (see the module
    // header), so the manual `updated_at = NOW()` in the next two statements is
    // REQUIRED - do not sweep it away (ws F36).
```

- [ ] **DO NOT** touch any UPDATE outside db.rs, and do not add `updated_at = NOW()` anywhere.
- [ ] Sanity-check the sweep with:
      `grep -n "updated_at = NOW()\|updated_at = timezone" /home/beefsack/Development/brdgme/rust/web/src/db.rs | awk -F: '$1<3200'`
      (Before this task the same grep returns **27** lines. `3200` is a safe upper bound: `mod tests {` is at live :3140 and shifts *down*, never up, as strings collapse.)
      Expected output after this task: **exactly three** lines — the two `game_proposals` UPDATEs in `delete_game`, and `update_game_command_success`'s `games` UPDATE (which Task 3 handles).
- [ ] Add this regression test at the end of the `mod tests` block (`mod tests {` opens at live :3140; its closing `}` is the file's last line, live :6877). It is the sweep's proof: it asserts the trigger really does the work for the two tables the sweep touched most.

```rust
    /// ws F36: the manual `updated_at = NOW()` assignments were removed from
    /// every UPDATE against a trigger-maintained table (see the module header).
    /// This pins the trigger actually doing the work, for both `games` and
    /// `game_players`, so a future accidental removal of the trigger - or a
    /// re-added manual set - is caught here.
    #[sqlx::test]
    async fn update_updated_at_trigger_maintains_games_and_game_players(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        // Backdate both rows so any trigger-driven bump is unmistakable.
        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE game_players DISABLE TRIGGER update_game_players_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET updated_at = '2020-01-01 00:00:00' WHERE id = $1")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE game_players SET updated_at = '2020-01-01 00:00:00' WHERE game_id = $1")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE games ENABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE game_players ENABLE TRIGGER update_game_players_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        // mark_game_read UPDATEs game_players and no longer sets updated_at.
        mark_game_read(&pool, game.id, a.id).await.unwrap();
        let gp_updated: time::PrimitiveDateTime = sqlx::query_scalar(
            "SELECT updated_at FROM game_players WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            gp_updated.year() > 2020,
            "update_game_players_updated_at must bump updated_at without a manual set, got {gp_updated}"
        );

        // end_game UPDATEs games and no longer sets updated_at.
        end_game(&pool, game.id).await.unwrap();
        let g_updated: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            g_updated.year() > 2020,
            "update_games_updated_at must bump updated_at without a manual set, got {g_updated}"
        );
    }
```

**Verification checkpoint:**

- [ ] `cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings` — clean.
- [ ] `cargo fmt --all -- --check` — clean.
- [ ] The `grep` above prints exactly 3 lines.
- [ ] `.sqlx` regeneration is deferred to the final commit (Global Constraints); `SQLX_OFFLINE=true cargo check -p web --features ssr` **will fail** on the changed macro strings until then — that is expected at this checkpoint and is the reason the regeneration step is mandatory before the last commit.
- [ ] With the test containers up: `cargo test -p web --features ssr update_updated_at_trigger` passes; `cargo test -p web --features ssr db::tests` shows no new failures.

**Commit:** `refactor(db): drop dead manual updated_at sets, document trigger convention (ws F36, F42 partial)`

---

## Task 2: unify `is_user_admin` on `anyhow::Result` (ws F45)

**Problem:** `is_user_admin` (live :567) is the only public non-test function in db.rs returning `sqlx::Result<bool>`; every neighbour returns `anyhow::Result`. Callers that combine it with other db calls juggle two error types.

**Why this is safe:** there are exactly **20** call sites outside db.rs and every one of them compiles unchanged against `anyhow::Result<bool>`:

- **18 via `.map_err(internal("..."))`** — `game/server_fns.rs:285`, `:1353`, `:1372`; `admin.rs:640`, `:665`, `:700`, `:729`, `:748`, `:767`, `:790`, `:815`, `:834`, `:853`, `:879`, `:914`, `:942`, `:962`, `:985`. `internal` is declared `pub fn internal<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> ServerFnError` (`/home/beefsack/Development/brdgme/rust/web/src/error.rs:7`) and `anyhow::Error: Display`.
- **1 via `match`** — `game/export.rs:195-202` (`Ok(true) => {}`, `Ok(false) => FORBIDDEN`, `Err(e) => tracing::error!("...: {e}")`); `Display` again.
- **1 via `.await.unwrap()`** — `admin.rs:2201`, inside admin.rs's own `mod tests` (module opens at `admin.rs:2177`). `.unwrap()` needs `E: Debug`, and `anyhow::Error: Debug`, so this also compiles. It is a *test* call in **WP-37's** file; do not edit it.

**No file outside db.rs changes.**

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Replace the signature at live :567 and add the `?`-to-anyhow conversion:

```rust
pub async fn is_user_admin(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let row: Option<(bool,)> = sqlx::query_as("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(a,)| a).unwrap_or(false))
}
```

  (The body is unchanged except that `?` now converts `sqlx::Error` into `anyhow::Error` via the blanket `From` impl, which `Result` = `anyhow::Result` accepts.)
- [ ] Confirm no call site needs editing:
      `grep -rn "is_user_admin" /home/beefsack/Development/brdgme/rust/web/src --include=*.rs`
      Expect **21** hits: the definition at `db.rs:567` plus the 20 listed above. Every hit outside db.rs must be `.map_err(internal(...))`, a `match` on `Err(e)`, or `.await.unwrap()`. **If any hit does something else — specifically `?` inside a function returning `sqlx::Result`, or a `match`/`if let` on an `sqlx::Error` variant such as `sqlx::Error::RowNotFound` — STOP and report**, because those are the only two shapes that would stop compiling, and their presence would mean the caller inventory changed since this spec was written.

**Test plan:**

- [ ] Add at the end of `mod tests` (this is also F35 coverage for `is_user_admin`):

```rust
    /// ws F35 + ws F45: `is_user_admin` had no test at all, and now returns
    /// `anyhow::Result`. Covers all three outcomes including the fail-closed
    /// unknown-user case.
    #[sqlx::test]
    async fn is_user_admin_true_false_and_unknown_user(pool: PgPool) {
        let plain = make_user(&pool, "plain").await;
        let admin = make_user(&pool, "adminuser").await;
        sqlx::query("UPDATE users SET is_admin = true WHERE id = $1")
            .bind(admin.id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(is_user_admin(&pool, admin.id).await.unwrap());
        assert!(!is_user_admin(&pool, plain.id).await.unwrap());
        // Fail closed for a user id that does not exist.
        assert!(!is_user_admin(&pool, Uuid::new_v4()).await.unwrap());
    }
```

  Cases: admin row → `true`; non-admin row → `false`; absent row → `false` (not an error).

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean — this is the real proof the 19 call sites still compile.
- [ ] `cargo test -p web --features ssr is_user_admin_true_false` passes.

**Commit:** `refactor(db): is_user_admin returns anyhow::Result, add coverage (ws F45, ws F35)`

---

## Task 3: make `is_finished` sticky in `update_game_command_success` (ws F37)

**Problem:** live :1891 writes `is_finished = $2` **unconditionally** while `finished_at = COALESCE($3, finished_at)` preserves the old timestamp. A second command carrying `is_finished: false` against an already-finished game therefore produces `is_finished = false AND finished_at IS NOT NULL` — an incoherent row. The existing test `update_game_command_success_writes_finished_fields` (`#[sqlx::test]` at :4810, fn at :4811, closing `}` at :4890) drives exactly that sequence: first call `is_finished: true` (:4821-4839), second call `is_finished: false` (:4863-4882), then it asserts only `finished_at` (:4885-4889), enshrining the bug; its comment at :4858-4862 ends with a dangling "see report" reference.

**Fix and why this shape:** the two columns must agree on stickiness. `COALESCE($3, finished_at)` already means "a non-finish update never clears the finish"; `is_finished = ($2 OR is_finished)` says the same thing for the boolean. The legitimate un-finish path is `undo_game` (:1547), which explicitly writes `is_finished = $2` **and** `finished_at = NULL` together — it is unaffected and stays the single place a game can become unfinished. The alternative (clearing `finished_at` when `$2` is false) is rejected: it would let a stray non-finish command erase a real finish timestamp.

The `update_finished_at` trigger (001:448-452) fires only on false→true and is unaffected — note it *does* fire on the first (genuine) finish and overwrites `finished_at` with `now()`, which is pre-existing behaviour this task does not change. `rows_affected() == 0` still detects stale state, because the `WHERE id = $4 AND updated_at = $5` guard is untouched and the `update_games_updated_at` trigger still bumps `updated_at`.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Replace the `games` UPDATE inside `update_game_command_success` (live :1890-1899, i.e. `let update_result = sqlx::query!(` through `.await?;`) with — note this also completes Task 1 item 11:

```rust
    // `is_finished` is sticky: a finished game stays finished, matching
    // `COALESCE($3, finished_at)` on the timestamp column. Un-finishing is
    // `undo_game`'s job (it writes is_finished AND finished_at = NULL
    // together); allowing a stray non-finish command to flip the flag here
    // produced `is_finished = false` with a non-NULL `finished_at` (ws F37).
    // `updated_at` is maintained by the update_games_updated_at trigger, so
    // the optimistic-concurrency guard below still sees a changed value.
    let update_result = sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = ($2 OR is_finished), finished_at = COALESCE($3, finished_at) WHERE id = $4 AND updated_at = $5",
        new_game_state,
        status.is_finished,
        finished_at,
        game_id,
        expected_updated_at
    )
    .execute(&mut *tx)
    .await?;
```

- [ ] Edit the existing test `update_game_command_success_writes_finished_fields` (live :4810-4890). Replace the dangling five-line comment block at :4858-4862 (it currently begins `// The COALESCE only guards the case where the finished_at param is NULL` and ends `// preserving it - this differs from the plan's phrasing, see report.`) and add the missing assertion. The comment becomes:

```rust
        // Second command carries is_finished = false. Finish is sticky in both
        // columns: `is_finished` stays true (`($2 OR is_finished)`) and
        // `finished_at` is preserved by the COALESCE. When is_finished = true
        // the call passes Some(now), so a genuine second finish DOES advance
        // finished_at - only `undo_game` un-finishes a game (ws F37).
```

- [ ] Immediately after the existing `assert_eq!(ge_after_2.game.finished_at, Some(first_finished_at), "COALESCE preserves finished_at when the new value is NULL");` at :4885-4889 (i.e. between it and the test's closing `}` at :4890), add:

```rust
        assert!(
            ge_after_2.game.is_finished,
            "is_finished must stay true once set; a non-finish command must not \
             produce is_finished = false with a non-NULL finished_at (ws F37)"
        );
```

**Test plan:**

- Existing, edited: `update_game_command_success_writes_finished_fields` — finish a 2-player game (`is_finished: true`, placings `[1,2]`) → `is_finished == true`, `finished_at == Some(t0)`; then a second command with `is_finished: false, whose_turn: vec![0], placings: vec![]` → `finished_at == Some(t0)` (unchanged assertion) **and** `is_finished == true` (new assertion, would fail before the fix).
- Existing, must stay green **unmodified** (they prove the fix did not break the normal paths): the `undo_game` test that asserts `!ge_after.game.is_finished` and `ge_after.game.finished_at.is_none()` after an undo (live :5493-5494 — this is the one that proves `undo_game` is still the un-finish path); the `concede_game` test that asserts `is_finished` and `finished_at.is_some()` (live :5539-5540); `update_game_command_success_keeps_first_undo_stash_in_a_run` (live :4893+); `update_game_command_success_mid_turn_keeps_last_turn_at` (live :4759-4807); every `find_finished_game_summaries` test.
- Command: `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr update_game_command_success` (with the test containers running, or via `scripts/rust-test.sh`).

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr update_game_command_success` — all pass.
- [ ] `cargo test -p web --features ssr undo_game` — all pass.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] The `grep` from Task 1 now prints exactly **2** lines (the two `game_proposals` UPDATEs).

**Commit:** `fix(db): make is_finished sticky on the command path (ws F37)`

---

## Task 4: document `is_turn_at`'s last-activity semantics (ws F44)

**Problem the finding raised:** live :1921 `let is_turn_at = if is_turn { now } else { p_is_turn_at };` re-stamps `is_turn_at` on *every* command for a player who remains on turn, while the `update_is_turn_at` trigger only stamps false→true transitions. The finding read this as two mechanisms fighting.

**Why no code change (the recommendation is overturned):** the same UPDATE also sets `turn_reminder_sent_at = NULL` (live :1934), and the only consumer that treats `is_turn_at` as an elapsed-time threshold is the turn-reminder sweep, whose predicate (`/home/beefsack/Development/brdgme/rust/web/src/email/sweep.rs:62-68`) is `gp.is_turn = true AND gp.is_eliminated = false AND gp.turn_reminder_sent_at IS NULL AND gp.is_turn_at < NOW() - ($1 || ' seconds')::interval AND gp.game_bot_id IS NULL AND u.reminder_emails_enabled = true` — the two clauses that matter here are `turn_reminder_sent_at IS NULL` (sweep.rs:64) and the `is_turn_at` threshold (sweep.rs:65). Resetting both fields in one statement is internally consistent and gives the desirable behaviour: a player mid-multi-action-turn who just acted does not get nagged. The other consumer, `find_active_turn_games` (live :3101-3119, `ORDER BY gp.is_turn_at ASC`), orders the digest by *least recent activity*, which is also the useful ordering. Renaming the column would need a migration — disproportionate for a nit. So: **document, do not change.**

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Insert this comment immediately above live :1921 (`let is_turn_at = ...`), leaving the assignment itself byte-identical:

```rust
        // `is_turn_at` is LAST TURN ACTIVITY, not "turn started": it is
        // re-stamped on every command by a player who is still on turn, in the
        // same statement that clears `turn_reminder_sent_at` below. That pairing
        // is deliberate - the turn-reminder sweep gates on
        // `turn_reminder_sent_at IS NULL AND is_turn_at < NOW() - threshold`
        // (email/sweep.rs:64-65), so a player mid-multi-action-turn who just
        // acted is not nagged. `find_active_turn_games` orders the switch digest
        // by the same field, i.e. least-recently-active first. The
        // `update_is_turn_at` trigger (migrations/001:454-458) only covers the
        // false -> true transition and is not a substitute for this write
        // (ws F44).
```

**Test plan:** documentation only; no behaviour changes, so no new test is warranted and none of the existing suite may move. The behaviour being documented is already pinned by:
- `update_game_command_success_mid_turn_keeps_last_turn_at` (`#[sqlx::test]` at live :4759, fn :4760, closing `}` :4808), which asserts `p0.game_player.is_turn` and `last_turn_at == last_turn_at_before` — i.e. neither the `update_is_turn_at` nor the `update_last_turn_at` trigger fires when there is no `is_turn` transition, and
- the reminder-sweep tests in `/home/beefsack/Development/brdgme/rust/web/src/email/sweep.rs` (out of scope — do not edit).

State this in the commit body so a reviewer does not read the comment-only change as an untested db.rs change (CODING.md's rule is about behaviour changes; this task has none, and Task 1's test lands in the same package).

**Verification checkpoint:**

- [ ] `git diff --stat` for this task shows db.rs with **additions only**, all comment lines.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `docs(db): record is_turn_at last-activity semantics (ws F44)`

---

## Task 5: mechanical cleanups and sentinel documentation (ws F43, ws F46, ws F49, ws F50)

**Problem:** four independent small items, batched because none changes behaviour and each is a few lines.

- **ws F49** — `choose_colors` (live :977, inside the `'outer: loop {` that opens at :976) clones the entire `rem_prefs` vec on every outer-loop pass. The loop body mutates only `assigned` and `remaining`, so the clone buys nothing.
- **ws F50** — `apply_rating_changes`' all-pairs loop (header live :1802-1806, inner `for` at :1807) uses `.iter().take(rated_players.len().saturating_sub(1)).enumerate()` + `.skip(a_index + 1)`. Correct but obscure; the `take` is redundant because the last index yields an empty tail.
- **ws F43** — `build_game_type_user` (`#[cfg]` at live :56, `#[allow(clippy::too_many_arguments)]` at :58, fn :59, closing `}` :110) returns a synthetic `GameTypeUser` at :99-108 with `id: Uuid::nil()` and rating 1200 when the LEFT JOIN produced NULLs. No caller distinguishes it; the sentinel is undocumented.
- **ws F46** — `generate_unique_username` (doc comment live :857-861, `#[cfg]` :862, fn :863) SELECTs availability at :876-883 and the caller INSERTs; the window is closed only by `users_name_lower_key`.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] **ws F49.** Replace live :977 `for (pos, pref) in rem_prefs.clone() {` with `for (pos, pref) in &rem_prefs {` and adjust the two `pos` uses inside that loop body — `assigned.contains_key(&pos)` on live :978 becomes `assigned.contains_key(pos)`, and `assigned.insert(pos, remaining.remove(idx))` on live :983 becomes `assigned.insert(*pos, ...)` — so that live :977-987 reads exactly:

```rust
        // Iterate by reference: the body mutates only `assigned` and
        // `remaining`, never `rem_prefs`, so the old per-pass clone of the
        // whole vec bought nothing (ws F49).
        for (pos, pref) in &rem_prefs {
            if assigned.contains_key(pos) || pref.is_empty() {
                continue;
            }
            let want_color = &pref[0];
            if let Some(idx) = remaining.iter().position(|c| c == want_color) {
                assigned.insert(*pos, remaining.remove(idx));
            }
            if remaining.is_empty() {
                break 'outer;
            }
        }
```

  `LocPref` is `type LocPref = (usize, Vec<String>)` (live :906), so iterating `&rem_prefs` binds `pos: &usize` and `pref: &Vec<String>`; `contains_key` takes `&usize` directly, `pref.is_empty()` and `&pref[0]` work through auto-deref, and `insert` needs the `*pos`. `if let Some(new_prefs) = remove_highest_prefs(&rem_prefs)` on live :989 and the `rem_prefs = new_prefs;` on :990 are outside the `for`, so the immutable borrow has ended by then. If the borrow checker complains, you changed something else — do not "fix" it by reintroducing the clone.

- [ ] **ws F50.** Replace the outer loop header at live :1802-1807 (the five lines `for (a_index, a) in rated_players` / `.iter()` / `.take(rated_players.len().saturating_sub(1))` / `.enumerate()` / `{`, plus the inner `for b in rated_players.iter().skip(a_index + 1) {`) with:

```rust
    // Each unordered pair exactly once: index `i` against the tail slice.
    // (Was `.take(len - 1).enumerate()` + `.skip(a_index + 1)`; the `take` was
    // redundant because the last index yields an empty tail - ws F50.)
    for (i, a) in rated_players.iter().enumerate() {
        for b in &rated_players[i + 1..] {
```

  Keep the loop body (live :1808-1818, `let a_place = ...` through `*rating_changes.entry(b.position).or_insert(0) -= change;`) byte-identical, and keep the two closing braces at :1819-1820. `a_index` is no longer referenced anywhere; `rated_players` is a `Vec<RatedPlayer>` (built by `.push` at :1780-1784) so `&rated_players[i + 1..]` is a valid slice and `b` stays `&RatedPlayer`.

- [ ] **ws F43.** Extend `build_game_type_user`'s definition with a doc comment. Insert it **above the `#[cfg(feature = "ssr")]` at live :56** — note there is a second attribute, `#[allow(clippy::too_many_arguments)]` at :58 with a `// Splitting these into a params struct...` comment at :57 between the `#[cfg]` and the `fn` at :59; leave all three lines where they are.

```rust
/// Builds a `GameTypeUser` from LEFT-JOINed columns, synthesizing a default row
/// when the join produced NULLs (a player who has not been rated in this game
/// type yet).
///
/// **The synthetic row is marked by `id == Uuid::nil()`** and carries
/// `rating = peak_rating = 1200`, matching the `game_type_users.rating` column
/// default, with `last_game_finished_at = None`, `created_at`/`updated_at` set
/// to the caller's `default_ts`, and `user_id = default_user_id` (also
/// `Uuid::nil()` when the caller had no user id, i.e. a bot slot). That is
/// deliberate - new
/// players start at 1200 and the render path wants a value, not an `Option` -
/// but it means callers cannot tell "no rating row yet" from "a real row
/// sitting at 1200" except via the nil id. No caller reads `id` today (the
/// only field consumed off this struct outside db.rs is `rating`, at
/// `game/server_fns.rs:369`); if one ever needs the distinction, change the
/// return type to `Option<GameTypeUser>` rather than adding nil-id checks at
/// call sites (ws F43).
```

- [ ] **ws F46.** Extend `generate_unique_username`'s existing doc comment (live :857-861, the five `///` lines ending `Takes a connection so it can run inside callers' transactions.`) by appending these lines after :861 and before the `#[cfg(feature = "ssr")]` at :862:

```rust
/// **Race note (ws F46):** the availability SELECT and the caller's INSERT are
/// separate statements, so a concurrent transaction can claim the same
/// generated name in between. The `users_name_lower_key` unique index
/// (migrations/009_username_rules.sql:41) is the actual guarantee; the loser
/// gets a 23505 that surfaces as a failed account/game creation. Retrying here
/// is not possible: every caller runs this inside an open transaction
/// (auth/server.rs:439, game/import.rs:181, proposals.rs:902,
/// db.rs `create_game_with_users_tx`), where a 23505 aborts the whole
/// transaction, so a retry would need SAVEPOINT plumbing in four modules. The
/// petname space plus the 100-attempt loop makes a collision vanishingly rare.
```

**Test plan:**

- **ws F49 / ws F50** are behaviour-preserving refactors of code that IS already tested; the existing tests are the verification and must pass **unmodified**: the `choose_colors` table-driven tests (grep `choose_colors` in `mod tests`) and the ELO/rating tests (grep `apply_rating_changes`, `rating_change`, `peak_rating` in `mod tests`).
- **ws F43** is documentation of behaviour already pinned by the existing test `find_game_extended_missing_game_type_user_defaults_to_1200` (`#[sqlx::test]` at live :4593, fn :4594, closing `}` :4623), which deletes the auto-created `game_type_users` row and asserts `rating == 1200`, `peak_rating == 1200` and `game_type_id == game_type_id` via `find_game_extended`. Add one assertion to that existing test to pin the sentinel the comment now promises — insert after the existing `assert_eq!(human.game_type_user.rating, 1200);` at :4620 (i.e. before the `peak_rating` assertion at :4621, or after :4622; either position is fine):

```rust
        assert_eq!(
            human.game_type_user.id,
            Uuid::nil(),
            "synthetic default rating row must be marked by a nil id (ws F43)"
        );
```

- **ws F46** is documentation; `generate_unique_username` gets its first real test in Task 11.
- Commands: `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr choose_colors` and `cargo test -p web --features ssr rating`.

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr choose_colors` — all pass, unmodified.
- [ ] `cargo test -p web --features ssr rating` — all pass, unmodified.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean (clippy will flag a leftover unused `a_index` if the F50 edit was partial).
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `refactor(db): drop choose_colors clone, clarify pair loop, document sentinels (ws F49, F50, F43, F46)`

---

## Task 6: replace the text-built interval with `make_interval` (ws F47)

**Problem:** `delete_expired_unverified_emails` (`#[cfg]` live :3123, fn :3124, body :3128-3136, closing `}` :3137) builds its cutoff as `NOW() - ($1 || ' seconds')::interval` (the whole `WHERE` is on one line, :3131) and binds `secs.to_string()` (:3133). This is **not** an injection risk — `$1` is a bound parameter whose value is `i64::to_string()` of `std::time::Duration::as_secs()`, so it can only ever be ASCII digits, and Postgres concatenates after binding — but it round-trips an integer through text and re-parses it as an interval, which is a needless typing detour and a bad pattern to copy.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Replace the body of `delete_expired_unverified_emails` (live :3128-3136, i.e. `let secs = ...` through `Ok(res.rows_affected())`, leaving the signature at :3124-3127 and the closing `}` at :3137) with:

```rust
    let secs = threshold.as_secs() as i64;
    let res = sqlx::query(
        // `make_interval(secs => ...)` instead of the older
        // `($1 || ' seconds')::interval` idiom: the parameter stays an integer
        // instead of being formatted to text and re-parsed as an interval
        // (ws F47). Not an injection fix - `$1` was always a bound parameter.
        "DELETE FROM user_emails
         WHERE verified_at IS NULL
           AND created_at < NOW() - make_interval(secs => $1::double precision)",
    )
    .bind(secs)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
```

  `make_interval`'s `secs` parameter is `double precision`, hence the explicit cast on the bound `int8`. This is a plain (non-macro) query, so it does not add or change a `.sqlx` entry.
- [ ] **Do not** touch the four sibling sites in other files (`proposals.rs:725,755,819`, `email/sweep.rs:65`) — see Non-Goals and the routing note.

**Test plan:**

- Existing, must pass **unmodified** — it is the real verification and already covers the boundary: `expiry_cleanup_deletes_only_expired_unverified` (`#[sqlx::test]` at live :6771, fn :6772) inserts three rows (unverified created 48h ago; unverified created now; verified created 48h ago), calls with `Duration::from_secs(86400)`, and asserts exactly 1 deletion plus which addresses survive. Note the fixture uses raw SQL `NOW() - interval '48 hours'`, which is unaffected by this change.
- Add one case for the degenerate threshold, appended at the end of `mod tests`:

```rust
    /// ws F47: `make_interval(secs => 0)` must behave like the old
    /// `('0' || ' seconds')::interval` - i.e. a zero threshold deletes every
    /// unverified row (created_at is strictly in the past) and still spares
    /// verified ones.
    #[sqlx::test]
    async fn delete_expired_unverified_emails_zero_threshold(pool: PgPool) {
        let user = make_user(&pool, "sweeper").await;
        let unverified = format!("u-{}@example.com", Uuid::new_v4());
        let verified = format!("v-{}@example.com", Uuid::new_v4());
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user.id)
            .bind(&unverified)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user.id)
        .bind(&verified)
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_expired_unverified_emails(&pool, std::time::Duration::from_secs(0))
            .await
            .unwrap();
        assert_eq!(deleted, 1, "zero threshold must delete the unverified row");
        let remaining = list_user_emails(&pool, user.id).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].email, verified);
    }
```

  `list_user_emails` returns `Vec<UserEmailRow>` and `UserEmailRow` (live :2895-2901) has fields `id: Uuid`, `email: String`, `is_primary: bool`, `verified_at: Option<time::PrimitiveDateTime>` — `remaining[0].email` is correct as written.
- Command: `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr delete_expired_unverified_emails`

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr delete_expired_unverified_emails` — both tests pass.
- [ ] `grep -n "seconds')::interval" /home/beefsack/Development/brdgme/rust/web/src/db.rs` returns nothing.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.

**Commit:** `refactor(db): make_interval instead of text-built interval (ws F47)`

---

## Task 7: `send_friend_request` — serialize the pair and add the self-request no-op (ws F39, ws F48)

**Problem (two findings, one function, one commit — F48's fix changes a test that F39's fix must keep green):**

- **ws F39.** `send_friend_request` (`#[cfg]` live :2054, fn :2055, closing `}` :2102) reads the pair row (:2067-2075) then, in the `None` arm of `match row {` (:2076), INSERTs (:2078-2082) at READ COMMITTED. `friends_pair_key` is `UNIQUE (LEAST(source,target), GREATEST(source,target))` (`010_friends.sql:7-9`), so if A→B and B→A arrive concurrently and both read "no existing row", the loser's INSERT raises a raw 23505 instead of taking the documented mutual-intent auto-accept branch.
- **ws F48.** Self-friending is caught only by `friends_check` (001:114), surfacing as a generic DB error. The server fn already rejects it first with a proper message (`friends.rs:170-172`), so the db layer's job is only to be a coherent no-op like its other silent paths.

**Fix and why this shape:**

- For F39, a **transaction-scoped advisory lock keyed on the ordered pair**, taken as the first statement, makes the read-then-insert atomic against the opposite direction. `INSERT ... ON CONFLICT` is rejected: inferring the conflict target on a two-expression unique index requires the inference expressions to match `LEAST(...)`/`GREATEST(...)` verbatim, and `DO NOTHING` would swallow the auto-accept that the reverse-row branch exists to perform. `SERIALIZABLE` is rejected: it would push retry logic onto every caller.
- `hashtext()` gives two `integer`s from the ordered UUID pair for `pg_advisory_xact_lock(int4, int4)`. Both directions of the same pair compute the same key, so they serialize; **no deadlock is possible** because only one lock is ever taken and both callers take the same one. The lock is released at commit/rollback, including the early-return paths (which drop `tx` → rollback).
- For F48, an early `Ok(())` **before** the transaction begins.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Extend the doc comment above `send_friend_request` (live :2049-2053, the five `///` lines starting `/// D1 lifecycle.` and ending `/// (D7): the requester must not be able to distinguish any of these.`) by appending, after :2053 and before the `#[cfg(feature = "ssr")]` at :2054:

```rust
/// Self-requests are a silent no-op (the `friends_check` CHECK constraint
/// stays as the backstop, and `friends.rs`' server fn rejects them with a real
/// user error before we get here) - ws F48.
///
/// The whole read-then-insert runs under a transaction-scoped advisory lock on
/// the ORDERED pair, so two opposite-direction requests serialize and the
/// second one takes the mutual-intent auto-accept branch instead of colliding
/// with the `friends_pair_key` expression index (010_friends.sql:7-9) and
/// returning a raw 23505 - ws F39.
```

- [ ] Insert the self-request guard as the first statement of the function body (immediately after `pub async fn send_friend_request(pool: &PgPool, source: Uuid, target: Uuid) -> Result<()> {` at live :2055), **before** the `let mut tx = pool.begin().await?;` at live :2056:

```rust
    if source == target {
        // Silent no-op, matching this function's other silent paths (ws F48).
        return Ok(());
    }
```

- [ ] Insert the advisory lock immediately after the `let mut tx = pool.begin().await?;` at live :2056 — i.e. **before** the target-blocked-source check at :2057-2066, so it really is the transaction's first statement:

```rust
    // Serialize both directions of this unordered pair for the duration of the
    // transaction, so the read-then-insert below cannot race the opposite
    // direction into a raw `friends_pair_key` 23505 (ws F39). Same key from
    // either direction, and it is the only lock taken, so no deadlock is
    // possible. Released on commit or rollback.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(
           hashtext(LEAST($1::uuid, $2::uuid)::text),
           hashtext(GREATEST($1::uuid, $2::uuid)::text))",
    )
    .bind(source)
    .bind(target)
    .execute(&mut *tx)
    .await?;
```

- [ ] **Do not change `match row {` (live :2076) or any of its three arms** (`None => {}` :2077-2083, `Some(r) if r.source_user_id == source => {}` :2086, `Some(r) => {}` :2089-2099) **or the `tx.commit().await?;` at :2100.** Their behaviour — insert, silent no-op on an existing outgoing row, auto-accept a reverse non-accepted row — is exactly what the lock now makes reliable.
- [ ] Rewrite the existing test `self_request_rejected_by_db_check` (`#[sqlx::test]` at live :3492, fn :3493, body `let a = make_user(&pool, "alice").await;` :3494 and `assert!(send_friend_request(&pool, a.id, a.id).await.is_err());` :3495, closing `}` :3496) — this is one of the two sanctioned test edits. Replace all five lines with:

```rust
    /// ws F48: a self-request is a silent application-level no-op. The
    /// `friends_check` CHECK constraint (migrations/001:114) remains the
    /// backstop and is asserted directly below, so the guard cannot be
    /// mistaken for the DB's protection going away.
    #[sqlx::test]
    async fn self_request_is_silent_no_op(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        send_friend_request(&pool, a.id, a.id)
            .await
            .expect("self-request must be a silent Ok, not an error");
        assert_eq!(
            count_rows(&pool, "friends").await,
            0,
            "self-request must not write a friends row"
        );
        // The DB CHECK still rejects a self row inserted directly.
        let direct = sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
            .bind(a.id)
            .bind(a.id)
            .execute(&pool)
            .await;
        assert!(
            direct.is_err(),
            "friends_check must still reject a self row (ws F48 backstop)"
        );
    }
```

  `count_rows` is the existing test helper defined at live :6529-6534 (`async fn count_rows(pool: &PgPool, table: &str) -> i64`). Note it is defined **after** this test in the file; that is fine — Rust item order inside a module does not matter.

**Test plan:**

- Edited: `self_request_is_silent_no_op` — `(a, a)` → `Ok(())`, zero `friends` rows, and a direct INSERT still errors.
- New, for F39. Append at the end of `mod tests`:

```rust
    /// ws F39: two opposite-direction requests must end in the mutual-intent
    /// accepted state regardless of interleaving. A single-connection test
    /// cannot force the true concurrent interleaving, so this asserts the
    /// serialized outcome plus that the advisory lock is re-entrant for the
    /// same pair within one session (taking it twice must not deadlock or
    /// change the result), which is what the pooled server does across
    /// sequential requests.
    #[sqlx::test]
    async fn opposite_direction_requests_auto_accept_under_pair_lock(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;

        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true))),
            "B->A after A->B must auto-accept the single pair row"
        );
        assert_eq!(
            count_rows(&pool, "friends").await,
            1,
            "the pair-unique index must still leave exactly one row"
        );

        // Re-sending in either direction stays a no-op and never errors.
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(count_rows(&pool, "friends").await, 1);
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
    }
```

  `friend_row_state` is the existing helper **defined at live :3364-3375**: `async fn friend_row_state(pool: &PgPool, a: Uuid, b: Uuid) -> Option<(Uuid, Option<bool>)>`, selecting `source_user_id, has_accepted` for the pair in either direction — so `Some((a.id, Some(true)))` means "the surviving row was created by A and is accepted", which is exactly what the existing test at live :3463-3466 already asserts for this scenario.
- Existing, must pass **unmodified**: every other `send_friend_request` test in the `// --- #30 friends lifecycle ---` block (live :3377 onward) and `pair_unique_index_rejects_reverse_duplicate` (`#[sqlx::test]` at live :3470, fn :3471, closing `}` :3490 — raw SQL INSERTs, no `send_friend_request` call, so it is unaffected by both the lock and the self-guard). Enumerate them with `grep -n "send_friend_request(" /home/beefsack/Development/brdgme/rust/web/src/db.rs | awk -F: '$1>3140'`; note `accept_friends` (live :3699-3702) also calls `send_friend_request` twice, so every test using that fixture is in scope of this task's regression risk.
- Command: `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr friend`

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr friend` — all pass, with only the one rewritten test differing from before.
- [ ] `cargo test -p web --features ssr db::tests` — no other failures.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] If `hashtext` is unavailable (it is a built-in internal Postgres function and is expected to work on Postgres 18), the test fails with `function hashtext(text) does not exist`. **In that case STOP and report** — do not silently swap in an md5-based key.

**Commit:** `fix(db): serialize friend-request pairs, silent self-request no-op (ws F39, ws F48)`

---

## Task 8: inline the visibility predicate into `friend_recent_visible_game` (ws F40)

**Problem:** `friend_recent_visible_game` (`#[cfg]` live :2493, fn :2494, closing `}` :2520) fetches up to `scan_limit` candidate games into `candidates` (:2500-2513) and then issues one `is_game_visible_to_user` query per candidate in the `for` loop at live :2514-2518. `index.rs:52` calls it once per friend with `scan_limit = 10`, inside `for (friend_id, friend_name) in friends` (`index.rs:51`), so the logged-in index can issue up to 10 extra round trips per friend.

**Fix and why this shape:**

- The predicate is inlined **verbatim** from `is_game_visible_to_user`'s query string (live :2412-2424), with `$1`→`g.id` for the game and `$2`→`$3` for the viewer, and the correlated `game_players` alias renamed to `gp2`/`v` so it does not collide with the outer `gp`. Note the inner `JOIN users u ON u.id = gp2.user_id` is an INNER join, which is what makes bot rows (`user_id IS NULL`) unable to block visibility — preserve it exactly.
- The scan window is preserved **exactly** by evaluating the predicate as a projected boolean inside a derived table that keeps `ORDER BY g.updated_at DESC, g.id LIMIT $2`, then filtering outside. A naive `WHERE <predicate> ... LIMIT $2` would change behaviour: today the function returns `None` when none of the `scan_limit` most recent games is visible, whereas filter-then-limit would reach further back and start surfacing older games. That may be desirable, but it is a product change and is not this task's job.
- Verification's caveat ("inlining duplicates a deliberately centralized predicate") is answered by cross-reference comments in **both** functions plus a drift-guard test asserting they agree.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Append to `is_game_visible_to_user`'s doc comment (live :2402-2404, the three `///` lines ending `A 'private' player blocks all non-self viewing. Bots never block.`), before the `#[cfg(feature = "ssr")]` at :2405:

```rust
/// **This predicate is duplicated once**, inlined into
/// `friend_recent_visible_game` to avoid a per-candidate round trip (ws F40).
/// The two are kept in step by
/// `friend_recent_visible_game_matches_is_game_visible_to_user`; if you change
/// the rule here, change it there and that test will tell you if you missed a
/// case.
```

- [ ] Replace `friend_recent_visible_game` in full (live :2493-2520, from its bare `#[cfg(feature = "ssr")]` — it currently has no doc comment — through its closing `}`) with:

```rust
/// The friend's most recently updated game that `viewer_id` may see, scanning
/// only the `scan_limit` most recent (so an old visible game does not surface
/// when everything recent is hidden - the pre-inlining behaviour, preserved).
///
/// The visibility rule is `is_game_visible_to_user`'s predicate, inlined so
/// this is one query instead of one query per candidate (ws F40). The derived
/// table applies the scan window first and the predicate is projected as
/// `visible`, which keeps the window semantics identical. Keep in step with
/// `is_game_visible_to_user`; the
/// `friend_recent_visible_game_matches_is_game_visible_to_user` test asserts
/// the two agree.
#[cfg(feature = "ssr")]
pub async fn friend_recent_visible_game(
    pool: &PgPool,
    friend_user_id: Uuid,
    viewer_id: Uuid,
    scan_limit: i64,
) -> Result<Option<(Uuid, String, time::PrimitiveDateTime)>> {
    Ok(sqlx::query_as(
        "SELECT c.id, c.name, c.updated_at
         FROM (
           SELECT g.id, gt.name, g.updated_at,
                  (EXISTS(SELECT 1 FROM game_players v
                          WHERE v.game_id = g.id AND v.user_id = $3)
                   OR NOT EXISTS(
                     SELECT 1 FROM game_players gp2
                     JOIN users u ON u.id = gp2.user_id
                     WHERE gp2.game_id = g.id
                       AND NOT (
                         u.game_visibility = 'public'
                         OR (u.game_visibility = 'friends' AND EXISTS(
                           SELECT 1 FROM friends f WHERE f.has_accepted = TRUE
                             AND ((f.source_user_id = $3 AND f.target_user_id = u.id)
                               OR (f.target_user_id = $3 AND f.source_user_id = u.id))
                         ))
                       ))) AS visible
           FROM game_players gp
           JOIN games g ON g.id = gp.game_id
           JOIN game_versions gv ON gv.id = g.game_version_id
           JOIN game_types gt ON gt.id = gv.game_type_id
           WHERE gp.user_id = $1
           ORDER BY g.updated_at DESC, g.id
           LIMIT $2
         ) c
         WHERE c.visible
         ORDER BY c.updated_at DESC, c.id
         LIMIT 1",
    )
    .bind(friend_user_id)
    .bind(scan_limit)
    .bind(viewer_id)
    .fetch_optional(pool)
    .await?)
}
```

  Note this is a plain query (as before), so no `.sqlx` entry changes. The return type stays `Option<(Uuid, String, PrimitiveDateTime)>` and the three column positions are unchanged, so `index.rs` needs no edit.

**Test plan:**

- Existing, must pass **unmodified** (they pin both the ordering and the scan-window skip): `friend_recent_visible_game_returns_most_recent` (`#[sqlx::test]` live :3947, fn :3948), `friend_recent_visible_game_skips_private_player` (attr :3982, fn :3983 — the *newer* game has a private player and the *older* one must win), `friend_recent_visible_game_returns_none_when_no_games` (attr :4025, fn :4026).
- New drift guard. Append at the end of `mod tests`:

```rust
    /// ws F40: the inlined predicate must agree with `is_game_visible_to_user`
    /// case for case. Four visibility tiers x one game each; for each case,
    /// "the function returned this game" must equal "the shared predicate says
    /// this game is visible".
    ///
    /// Each case gets its OWN `friend` user (and therefore its own one-game
    /// scan universe), so no rows have to be deleted between cases and the
    /// `scan_limit` window is unambiguous.
    #[sqlx::test]
    async fn friend_recent_visible_game_matches_is_game_visible_to_user(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let viewer = make_user(&pool, "viewer").await;

        // Four co-players, one per case: public (the default), 'friends' and a
        // friend of the viewer, 'friends' but NOT a friend of the viewer, and
        // 'private'.
        let co_public = make_user(&pool, "copublic").await;
        let co_friends_yes = make_user(&pool, "cofriendsyes").await;
        let co_friends_no = make_user(&pool, "cofriendsno").await;
        let co_private = make_user(&pool, "coprivate").await;
        set_game_visibility(&pool, co_friends_yes.id, "friends")
            .await
            .unwrap();
        set_game_visibility(&pool, co_friends_no.id, "friends")
            .await
            .unwrap();
        set_game_visibility(&pool, co_private.id, "private")
            .await
            .unwrap();
        accept_friends(&pool, viewer.id, co_friends_yes.id).await;

        for (case, co, expected_visible) in [
            ("public", co_public.id, true),
            ("friends_yes", co_friends_yes.id, true),
            ("friends_no", co_friends_no.id, false),
            ("private", co_private.id, false),
        ] {
            // A fresh friend per case. The friend stays at the 'public'
            // default so only `co` can hide the game.
            let friend = make_user(&pool, &format!("friend_{case}")).await;
            accept_friends(&pool, viewer.id, friend.id).await;

            let game = make_game_with_players(&pool, gv, friend.id, &[co], 0, &[0]).await;
            let via_predicate = is_game_visible_to_user(&pool, game.id, viewer.id)
                .await
                .unwrap();
            let via_inlined = friend_recent_visible_game(&pool, friend.id, viewer.id, 10)
                .await
                .unwrap()
                .map(|(id, _, _)| id)
                == Some(game.id);
            assert_eq!(
                via_predicate, expected_visible,
                "is_game_visible_to_user disagreed with the expected case {case}"
            );
            assert_eq!(
                via_inlined, via_predicate,
                "inlined predicate disagreed with is_game_visible_to_user for case {case}"
            );
        }
    }
```

  Notes for the implementer, all verified so you do not have to guess:
  - `make_user` (live :3282) takes `&str`, so `&format!("friend_{case}")` is fine, and every `#[sqlx::test]` gets its own database so the four `friend_*` names cannot collide with anything.
  - `make_game_with_players` (live :3326-3358) routes through `create_game_with_users`, which inserts into `games`, `game_players` and `game_type_users` only — **no `game_logs` row** — but this test never deletes anything, so FK order is moot.
  - `viewer` is never a player in any of these games, so the `EXISTS(... v.user_id = $3)` self-clause is false in every case and the test really exercises the all-players branch.
- Command: `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr friend_recent_visible_game`

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr friend_recent_visible_game` — four tests pass (three existing unmodified + the new one).
- [ ] `cargo test -p web --features ssr is_game_visible_to_user` — passes unmodified.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.

**Commit:** `perf(db): one query for friend_recent_visible_game, with drift guard (ws F40)`

---

## Task 9: record the `insert_game_logs_tx` batching decision (ws F41)

**Problem the finding raised:** `insert_game_logs_tx` (`#[cfg]` live :1226, fn :1227, closing `}` :1275) issues one INSERT per log (:1248-1259) plus one per log target (:1264-1268), sequentially awaited inside the caller's transaction. It also silently drops any `to` position with no matching `game_players` row, via `if let Some(&player_id) = pos_to_id.get(&pos)` at :1263 — that is the behaviour Task 11's Test 3 pins.

**Decision: do not batch.** The finding's own recommendation is conditional ("If profiling shows it matters, batch ...; otherwise leave") and verification did not strengthen it. `N` is the log count of one game command (single digits); the only callers are `update_game_command_success` (live :1957) and `create_game_logs` (live :1284). Batching would replace three compile-time-checked `query!` macros with `UNNEST`-based inserts whose `.sqlx` entries need regenerating and hand-checking, for no measured gain. Record the decision so the next reader does not re-litigate it.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

**Steps:**

- [ ] Insert above `insert_game_logs_tx`'s `#[cfg(feature = "ssr")]` attribute (live :1226; the function currently has no doc comment):

```rust
/// Inserts a command's logs and their per-player targets inside the caller's
/// transaction.
///
/// Deliberately row-at-a-time: 1 + N + M sequential statements, where N is the
/// number of logs produced by a single game command (single digits in practice)
/// and M their targets. Reviewed as ws F41 and left alone - batching via
/// `UNNEST` would trade three compile-time-checked `query!` macros for
/// hand-verified offline metadata with no measured benefit. Revisit if a game
/// ever emits logs in the hundreds per command, or if this shows up in a real
/// profile.
```

**Test plan:** comment only, no behaviour change. `insert_game_logs_tx` gets its first direct test in Task 11 (it is one of the 26 untested functions), which is where this task's CODING.md obligation is discharged; note that in the commit body.

**Verification checkpoint:**

- [ ] `git diff` for this task shows comment lines only.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `docs(db): record insert_game_logs_tx row-at-a-time decision (ws F41)`

---

## Task 10: test-quality nits (ws F51)

**Problem:** three bundled nits; part (1) is overturned, parts (2) and (3) hold.

- **(1) OVERTURNED — do not act.** `suggestions_exclude_blocked_and_self` (`#[sqlx::test]` live :4192, fn :4193, closing `}` :4210) allegedly never tests self-exclusion. It does: lines :4198-4206 call `make_game_with_players(&pool, version, me.id, &[blocked_by_me.id, blocked_me.id], 0, &[0])`, and `make_game_with_players` routes through `create_game_with_users` with `creator_id: me.id`, so `me` **is** a `game_players` row of that game. If `opponent_suggestions` stopped excluding the caller, the final `assert!(opponent_suggestions(&pool, me.id).await.unwrap().is_empty())` at :4209 would fail. **Do not rename the test and do not add a case.**
- **(2) CONFIRMED.** The two existing `is_game_visible_to_user` tests — `is_game_visible_to_user_friends_tier` (attr :3762, fn :3763, `}` :3789) and `is_game_visible_to_user_private_blocks_non_self` (attr :3791, fn :3792, `}` :3811) — cover games with exactly *one* non-public player, but not the two-`friends`-players case where the viewer is a friend of only one. The rule is `NOT EXISTS(a player who fails the check)`, so that viewer must NOT see the game. That is the case most likely to break under a future rewrite of the predicate.
- **(3) CONFIRMED.** `count_rows` (live :6529-6534) builds SQL with `format!`. It is test-only and every argument is a literal table name, so it is safe today; it is a copy-paste hazard if the pattern migrates to production code.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs` (test module only).

**Steps:**

- [ ] **(2)** Insert this test immediately after `is_game_visible_to_user_private_blocks_non_self` closes (live :3811, before the `// --- Unit B2 public index game selection ---` comment at :3813):

```rust
    /// ws F51(2): with TWO 'friends'-tier players, a viewer who is a friend of
    /// only one of them must NOT see the game - the rule is "no player fails
    /// the check", not "some player passes it".
    #[sqlx::test]
    async fn is_game_visible_to_user_friends_tier_requires_every_friends_player(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let half_friend = make_user(&pool, "cara").await;
        let both_friend = make_user(&pool, "dana").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        set_game_visibility(&pool, a.id, "friends").await.unwrap();
        set_game_visibility(&pool, b.id, "friends").await.unwrap();
        // cara is friends with `a` only; dana is friends with both.
        accept_friends(&pool, a.id, half_friend.id).await;
        accept_friends(&pool, a.id, both_friend.id).await;
        accept_friends(&pool, b.id, both_friend.id).await;

        assert!(
            !is_game_visible_to_user(&pool, game.id, half_friend.id)
                .await
                .unwrap(),
            "a viewer friends with only one of two 'friends' players must NOT see the game"
        );
        assert!(
            is_game_visible_to_user(&pool, game.id, both_friend.id)
                .await
                .unwrap(),
            "a viewer friends with every 'friends' player must see the game"
        );
    }
```

- [ ] **(3)** Replace `count_rows` (live :6529-6534) with the same body plus a scope warning:

```rust
    /// Test-only row counter. **The `format!`-built SQL is safe ONLY because
    /// every caller passes a hard-coded table-name literal.** Do not copy this
    /// pattern outside `mod tests`, and never pass a runtime value for `table`
    /// (ws F51(3)).
    async fn count_rows(pool: &PgPool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(pool)
            .await
            .unwrap()
    }
```

- [ ] Confirm every `count_rows` call passes a literal:
      `grep -n "count_rows(" /home/beefsack/Development/brdgme/rust/web/src/db.rs`
      If any call passes a variable, report it rather than fixing it silently.

**Test plan:**

- New: `is_game_visible_to_user_friends_tier_requires_every_friends_player` — cara (friend of `a` only) → `false`; dana (friend of both) → `true`.
- Existing, unmodified: `is_game_visible_to_user_friends_tier` (live :3762-3789), `is_game_visible_to_user_private_blocks_non_self` (live :3791-3811), `suggestions_exclude_blocked_and_self` (live :4192-4210 — **explicitly left alone**).
- Command: `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr is_game_visible_to_user`

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr is_game_visible_to_user` — four tests pass.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.

**Commit:** `test(db): 3-player friends-tier visibility case, count_rows scope warning (ws F51)`

---

## Task 11: close the coverage gap — 11 `#[sqlx::test]`s over 24 untested public functions (ws F35, the major)

**Problem, re-derived from live source (do not trust the finding's list):** cross-referencing every `pub fn`/`pub async fn` in db.rs:15-3138 against the whole test module (:3140-6877) **and** `rust/web/tests/{ssr_pages,nats_bot_eventing,websocket_hygiene}.rs` gives **27 public functions with zero references anywhere** (an earlier draft of this spec said 26; the list itself was right, the count was not):

`count_incoming_friend_requests`, `create_game_with_users_tx`, `create_pool`, `find_active_turn_games`, `find_enabled_bots`, `find_game`, `find_game_version`, `find_game_version_render_meta`, `find_game_version_rules`, `find_latest_non_deprecated_game_version`, `find_open_restart_proposal`, `find_open_restart_proposal_tx`, `find_user_id_by_name`, `generate_unique_username`, `get_pending_request_source`, `get_user`, `get_user_by_email`, `get_user_name`, `get_user_pref_colors`, `has_block_conn`, `insert_game_logs_tx`, `is_user_admin`, `mark_game_read`, `replacement_bot_available`, `set_user_name`, `set_user_pref_colors`, `should_hide_add_friend`.

That is 27 names; verify with the loop at the end of this task. (`is_user_admin` is covered by Task 2, so **26** remain for this task, of which 2 are deliberately excluded below → **24 covered here**.) Per the triage note, `choose_colors` and the ELO helpers are **excluded** from the finding's list — they are already tested and the finding was wrong about them.

**The cut rule (documented, so the remainder is a decision and not an oversight):**

1. Every untested public function gets at least one assertion, **except** where covering it needs external resources or would duplicate existing transitive coverage.
2. Functions with real logic (ordering, capping, retry loops, silent-shield semantics, fan-out writes) get a dedicated test with the interesting cases.
3. Trivial single-statement getters/setters are batched into round-trip tests — one test per cohesive group, not one per function. Twenty-five single-function tests would be busywork and would slow the suite for no extra signal.
4. **Excluded, with reasons:** `create_pool` (live :160) reads `DATABASE_URL` and connects outside the `#[sqlx::test]` per-test database — testing it would test the harness, and `migrations_apply_and_pool_connects` (live :3144-3145) already proves the pool works. `create_game_with_users_tx` (live :1029) is the body of `create_game_with_users` (live :1014, a thin wrapper) and is transitively exercised by ~100 tests through the `make_game_with_players` fixture (:3326-3358); a direct test would assert the same statements twice.
5. **Routed out, not skipped:** the 3+-player `concede_game` hard check that F35's recommendation bundles in is a behaviour change inside WP-40's function → **WP-40**. The unasserted secondary behaviours F35 mentions for `recent_games_for_index` (exclusion of games the user is not in) and `find_active_game_summaries` (ordering) are query-shape assertions on functions that **are** already tested and sit in **WP-52**'s performance-pass scope → **WP-52**.

Net: **11 new tests covering 24 functions** (26 remaining after Task 2, minus the 2 excluded). Together with Task 2's `is_user_admin` test that is **25 of the 27**.

**Files:** Modify `/home/beefsack/Development/brdgme/rust/web/src/db.rs` (test module only). Append all tests at the end of `mod tests`, before its final `}` (the file's last line, live :6877).

**Existing helpers you must reuse (read them first, do not re-invent), with verified signatures:**

| Helper | Live lines | Signature / behaviour you need |
|---|---|---|
| `make_user` | :3282-3294 | `async fn make_user(pool: &PgPool, name: &str) -> User` — inserts with `pref_colors = []`, returns the full `User` (so `.id` and `.name` are available) |
| `make_game_type_and_version` | :3298-3320 | `async fn (pool: &PgPool) -> (Uuid, Uuid)` = `(game_type_id, game_version_id)`; the game type gets `player_counts = [2,3,4]`, the version `name = "1.0.0"`, `uri = "http://localhost:0/mock"`, `is_public = true`, `is_deprecated = false` |
| `make_game_with_players` | :3326-3358 | `async fn (pool, game_version_id: Uuid, creator_id: Uuid, opponent_ids: &[Uuid], bot_count: usize, whose_turn: &[usize]) -> crate::models::game::Game` — positions are creator, then opponents in order, then bots; bots are inserted into `game_bots` with `bot_name: "easy"` and do **not** require a matching `bots` row |
| `accept_friends` | :3699-3702 | `async fn (pool: &PgPool, a: Uuid, b: Uuid)` — calls `send_friend_request` in both directions, leaving one accepted row |
| `friend_row_state` | :3364-3375 | `async fn (pool, a, b) -> Option<(Uuid, Option<bool>)>` = `(source_user_id, has_accepted)` for the pair in either direction |
| `count_rows` | :6529-6534 | `async fn (pool: &PgPool, table: &str) -> i64` |

**Fixture fact you must not forget:** `migrations/013_bot_efficacy.sql:41-44` **seeds the `bots` table** with three rows — `('easy', 0, ...)`, `('medium', 1, ...)`, `('hard', 2, ...)` — all `enabled = true` (column default) and all `can_replace_humans = false` (column default from `migrations/022_concede_bot_replacement.sql:16`). `bots.name` is `NOT NULL UNIQUE`. So in any `#[sqlx::test]` database, `find_enabled_bots` returns `["easy", "medium", "hard"]`, **never an empty vec**, and inserting a bot named `'easy'`/`'medium'`/`'hard'` raises 23505.

**Steps:**

- [ ] **Test 1 — `find_active_turn_games` (ordering + cap + exclusions).**

```rust
    /// ws F35: `find_active_turn_games` feeds the 22d switch digest and had no
    /// test. Covers oldest-turn-first ordering, the cap, and the three
    /// exclusions (not my turn, finished game, other user).
    ///
    /// Note: `game_players.is_turn_at` is `timestamp without time zone NOT
    /// NULL` (migrations/001_initial_schema.sql:193), so the query's `NULLS
    /// LAST` clause (db.rs:3112) is vestigial and cannot be exercised.
    /// Backdating `is_turn_at` alone does not disturb `is_turn`, so the
    /// `update_is_turn_at` trigger (001:454-458) does not fire and undo the
    /// fixture.
    #[sqlx::test]
    async fn find_active_turn_games_orders_oldest_turn_first_and_caps(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let me = make_user(&pool, "me").await;
        let other = make_user(&pool, "other").await;

        // Three games where it is my turn, with distinct is_turn_at values.
        let mut ids = Vec::new();
        for (i, day) in ["2026-01-03", "2026-01-01", "2026-01-02"].iter().enumerate() {
            let g = make_game_with_players(&pool, gv, me.id, &[other.id], 0, &[0]).await;
            sqlx::query("UPDATE game_players SET is_turn_at = $1::timestamp WHERE game_id = $2 AND user_id = $3")
                .bind(format!("{day} 00:00:00"))
                .bind(g.id)
                .bind(me.id)
                .execute(&pool)
                .await
                .unwrap();
            ids.push((i, g.id, *day));
        }
        // A game where it is NOT my turn.
        let not_my_turn = make_game_with_players(&pool, gv, me.id, &[other.id], 0, &[1]).await;
        // A finished game where it IS my turn.
        let finished = make_game_with_players(&pool, gv, me.id, &[other.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET is_finished = true WHERE id = $1")
            .bind(finished.id)
            .execute(&pool)
            .await
            .unwrap();

        let rows = find_active_turn_games(&pool, me.id, 10).await.unwrap();
        let got: Vec<Uuid> = rows.iter().map(|(g, _)| *g).collect();
        let by_day = |d: &str| ids.iter().find(|(_, _, day)| *day == d).unwrap().1;
        assert_eq!(
            got,
            vec![by_day("2026-01-01"), by_day("2026-01-02"), by_day("2026-01-03")],
            "must be ordered by is_turn_at ascending"
        );
        assert!(!got.contains(&not_my_turn.id), "must exclude games where it is not my turn");
        assert!(!got.contains(&finished.id), "must exclude finished games");

        // The returned game_player_id must be MY player row, not the opponent's.
        let (_, gp_id) = rows[0];
        let owner: Uuid = sqlx::query_scalar("SELECT user_id FROM game_players WHERE id = $1")
            .bind(gp_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(owner, me.id);

        // Cap.
        let capped = find_active_turn_games(&pool, me.id, 2).await.unwrap();
        assert_eq!(capped.len(), 2);
        assert_eq!(capped[0].0, by_day("2026-01-01"));

        // Another user sees none of my turns.
        assert!(find_active_turn_games(&pool, other.id, 10).await.unwrap().is_empty());
    }
```

- [ ] **Test 2 — `generate_unique_username`.**

```rust
    /// ws F35: `generate_unique_username` had no test. Asserts the result
    /// satisfies the D2 username rules and is unused, and that generating +
    /// claiming twice yields two distinct names.
    ///
    /// The taken-branch retry (line "if taken.is_none()") cannot be forced
    /// deterministically - the candidate comes from `petname`, so a test cannot
    /// pre-claim the exact name the next call will draw. This covers the loop's
    /// success path and the uniqueness contract, which is the part callers
    /// depend on.
    #[sqlx::test]
    async fn generate_unique_username_is_valid_and_unclaimed(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let first = generate_unique_username(&mut conn).await.unwrap();
        assert!(
            validate_username(&first),
            "generated name must satisfy the D2 rules: {first}"
        );
        assert!(
            find_user_id_by_name(&pool, &first).await.unwrap().is_none(),
            "generated name must be unused"
        );

        // Claim it, then generate again: the second name must differ and must
        // itself be claimable (the unique index would reject a duplicate).
        sqlx::query("INSERT INTO users (name, pref_colors) VALUES ($1, $2)")
            .bind(&first)
            .bind(Vec::<String>::new())
            .execute(&pool)
            .await
            .unwrap();

        let second = generate_unique_username(&mut conn).await.unwrap();
        assert!(validate_username(&second));
        assert_ne!(
            second.to_lowercase(),
            first.to_lowercase(),
            "must not hand back a name that is already claimed"
        );
        sqlx::query("INSERT INTO users (name, pref_colors) VALUES ($1, $2)")
            .bind(&second)
            .bind(Vec::<String>::new())
            .execute(&pool)
            .await
            .expect("second generated name must be insertable");
    }
```

- [ ] **Test 3 — `insert_game_logs_tx`.**

```rust
    /// ws F35: `insert_game_logs_tx` had no direct test (only empty-vec calls
    /// via `update_game_command_success`). Covers the log row fields, target
    /// fan-out by position, and that a `to` position with no matching player is
    /// silently dropped.
    #[sqlx::test]
    async fn insert_game_logs_tx_writes_logs_and_targets(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        let at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::March, 4).unwrap(),
            time::Time::from_hms(5, 6, 7).unwrap(),
        );
        let logs = vec![
            brdgme_cmd::api::CliLog {
                content: "public log".to_string(),
                at,
                public: true,
                to: vec![],
            },
            brdgme_cmd::api::CliLog {
                content: "private to both".to_string(),
                at,
                public: false,
                // position 9 does not exist and must be dropped silently.
                to: vec![0, 1, 9],
            },
        ];

        let mut tx = pool.begin().await.unwrap();
        insert_game_logs_tx(&mut tx, game.id, logs).await.unwrap();
        tx.commit().await.unwrap();

        let rows: Vec<(String, bool, time::PrimitiveDateTime, i64)> = sqlx::query_as(
            "SELECT gl.body, gl.is_public, gl.logged_at,
                    (SELECT COUNT(*) FROM game_log_targets t WHERE t.game_log_id = gl.id)
             FROM game_logs gl WHERE gl.game_id = $1 ORDER BY gl.body",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("private to both".to_string(), false, at, 2));
        assert_eq!(rows[1], ("public log".to_string(), true, at, 0));

        // Targets point at the two real player rows.
        let targeted: Vec<Uuid> = sqlx::query_scalar(
            "SELECT gp.user_id FROM game_log_targets t
             JOIN game_players gp ON gp.id = t.game_player_id
             JOIN game_logs gl ON gl.id = t.game_log_id
             WHERE gl.game_id = $1 ORDER BY gp.position",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(targeted, vec![a.id, b.id]);
    }
```

- [ ] **Test 4 — `mark_game_read`.**

```rust
    /// ws F35: `mark_game_read` had no test. Only the calling user's player row
    /// may be marked, and only in the named game.
    #[sqlx::test]
    async fn mark_game_read_marks_only_the_caller_in_that_game(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let g1 = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;
        let g2 = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;
        sqlx::query("UPDATE game_players SET is_read = false")
            .execute(&pool)
            .await
            .unwrap();

        mark_game_read(&pool, g1.id, a.id).await.unwrap();

        let read: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT game_id, user_id, is_read FROM game_players ORDER BY game_id, position",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        for (game_id, user_id, is_read) in read {
            let expected = game_id == g1.id && user_id == a.id;
            assert_eq!(
                is_read, expected,
                "is_read wrong for game {game_id} user {user_id} (g1={}, g2={})",
                g1.id, g2.id
            );
        }

        // Marking a game the user is not in is a no-op, not an error.
        let stranger = make_user(&pool, "stranger").await;
        mark_game_read(&pool, g1.id, stranger.id).await.unwrap();
    }
```

- [ ] **Test 5 — `should_hide_add_friend`.**

```rust
    /// ws F35: `should_hide_add_friend` had no test. The button hides when the
    /// viewer already has an outgoing row of ANY state (pending, declined,
    /// accepted - the D1 shield) and when an ACCEPTED reverse row exists, but
    /// NOT for a merely pending incoming request.
    #[sqlx::test]
    async fn should_hide_add_friend_covers_every_row_state(pool: PgPool) {
        let viewer = make_user(&pool, "viewer").await;
        let stranger = make_user(&pool, "stranger").await;
        let pending_out = make_user(&pool, "pendingout").await;
        let pending_in = make_user(&pool, "pendingin").await;
        let declined_out = make_user(&pool, "declinedout").await;
        let accepted = make_user(&pool, "accepted").await;

        send_friend_request(&pool, viewer.id, pending_out.id).await.unwrap();
        send_friend_request(&pool, pending_in.id, viewer.id).await.unwrap();
        send_friend_request(&pool, viewer.id, declined_out.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, declined_out.id)
            .await
            .unwrap()[0];
        respond_to_friend_request(&pool, req_id, declined_out.id, false)
            .await
            .unwrap();
        accept_friends(&pool, viewer.id, accepted.id).await;

        assert!(!should_hide_add_friend(&pool, viewer.id, stranger.id).await.unwrap());
        assert!(should_hide_add_friend(&pool, viewer.id, pending_out.id).await.unwrap());
        assert!(should_hide_add_friend(&pool, viewer.id, declined_out.id).await.unwrap());
        assert!(should_hide_add_friend(&pool, viewer.id, accepted.id).await.unwrap());
        assert!(
            !should_hide_add_friend(&pool, viewer.id, pending_in.id).await.unwrap(),
            "a pending INCOMING request must not hide the button - accepting it \
             by sending back is the documented mutual-intent path"
        );
    }
```

  If the last assertion fails, **stop and report it as a possible newly-discovered defect** rather than flipping the expectation: the predicate at live :2177-2185 matches an outgoing row of any state OR an accepted reverse row, so a pending incoming row should not match. Confirm by reading before changing anything.

- [ ] **Test 6 — `find_open_restart_proposal` and `find_open_restart_proposal_tx`.**

```rust
    /// ws F35: neither restart-proposal lookup had a test. Only `open`
    /// proposals count, the earliest wins, and the `_tx` variant must agree
    /// with the pool variant.
    #[sqlx::test]
    async fn find_open_restart_proposal_finds_earliest_open_only(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let owner = make_user(&pool, "owner").await;
        let other = make_user(&pool, "other").await;
        let old_game = make_game_with_players(&pool, gv, owner.id, &[other.id], 0, &[0]).await;

        // Helper as a plain async fn call, not a closure: no borrow-checker
        // gymnastics, and `game_proposals` only needs these five columns
        // (migrations/015_game_proposals.sql:5-15 - `status` is CHECKed against
        // 'open'/'started'/'cancelled', `created_at` is a bare `timestamp`).
        async fn insert_proposal(
            pool: &PgPool,
            gv: Uuid,
            owner_id: Uuid,
            old_id: Uuid,
            status: &str,
            created: &str,
        ) -> Uuid {
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO game_proposals
                   (game_version_id, owner_user_id, restarted_game_id, status, created_at)
                 VALUES ($1, $2, $3, $4, $5::timestamp) RETURNING id",
            )
            .bind(gv)
            .bind(owner_id)
            .bind(old_id)
            .bind(status)
            .bind(created)
            .fetch_one(pool)
            .await
            .unwrap()
        }

        // No proposal yet.
        assert!(find_open_restart_proposal(&pool, old_game.id).await.unwrap().is_none());

        let cancelled = insert_proposal(
            &pool, gv, owner.id, old_game.id, "cancelled", "2026-01-01 00:00:00",
        )
        .await;
        assert!(
            find_open_restart_proposal(&pool, old_game.id).await.unwrap().is_none(),
            "a cancelled proposal must not count"
        );

        let later_open = insert_proposal(
            &pool, gv, owner.id, old_game.id, "open", "2026-01-03 00:00:00",
        )
        .await;
        let earlier_open = insert_proposal(
            &pool, gv, owner.id, old_game.id, "open", "2026-01-02 00:00:00",
        )
        .await;
        assert_eq!(
            find_open_restart_proposal(&pool, old_game.id).await.unwrap(),
            Some(earlier_open),
            "earliest open proposal wins for a deterministic winner"
        );
        assert_ne!(earlier_open, later_open);
        assert_ne!(earlier_open, cancelled);

        // The _tx variant must agree.
        let mut tx = pool.begin().await.unwrap();
        assert_eq!(
            find_open_restart_proposal_tx(&mut tx, old_game.id).await.unwrap(),
            Some(earlier_open)
        );
        tx.rollback().await.unwrap();

        // An unrelated game has none.
        let unrelated = make_game_with_players(&pool, gv, owner.id, &[other.id], 0, &[0]).await;
        assert!(find_open_restart_proposal(&pool, unrelated.id).await.unwrap().is_none());
    }
```

  Both lookups are the same SQL (`db.rs:803-805` and `:817-819`): `SELECT id FROM game_proposals WHERE restarted_game_id = $1 AND status = 'open' ORDER BY created_at LIMIT 1`, so the "earliest open wins" and "cancelled does not count" assertions pin the real rule. `find_open_restart_proposal_tx` takes `&mut sqlx::PgConnection`; passing `&mut tx` where `tx: Transaction` works by deref coercion, the same way `insert_game_logs_tx(&mut tx, ...)` is already called at `db.rs:1957`.

- [ ] **Test 7 — `find_enabled_bots` and `replacement_bot_available`.**

```rust
    /// ws F35: neither bot lookup had a test. `find_enabled_bots` returns only
    /// enabled bots ordered by display_order; `replacement_bot_available`
    /// requires BOTH `enabled = true` AND `can_replace_humans = true`
    /// (column added by migrations/022_concede_bot_replacement.sql:16,
    /// defaulting to false).
    ///
    /// NOTE: migrations/013_bot_efficacy.sql:41-44 seeds three enabled bots
    /// ('easy' 0, 'medium' 1, 'hard' 2), all with can_replace_humans = false,
    /// so the baseline here is NOT an empty table and those three names are
    /// already taken (`bots.name` is UNIQUE).
    #[sqlx::test]
    async fn bot_lookups_respect_enabled_and_can_replace_humans(pool: PgPool) {
        // Seeded baseline.
        assert_eq!(
            find_enabled_bots(&pool).await.unwrap(),
            vec![
                "easy".to_string(),
                "medium".to_string(),
                "hard".to_string()
            ],
            "the three seeded bots, ordered by display_order"
        );
        assert!(
            !replacement_bot_available(&pool).await.unwrap(),
            "no seeded bot has can_replace_humans"
        );

        // A DISABLED bot is excluded from find_enabled_bots and must not make a
        // replacement available even though it can replace humans.
        sqlx::query(
            "INSERT INTO bots (name, display_order, enabled, can_replace_humans)
             VALUES ('offbot', 3, false, true)",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            find_enabled_bots(&pool).await.unwrap(),
            vec![
                "easy".to_string(),
                "medium".to_string(),
                "hard".to_string()
            ],
            "a disabled bot must be excluded"
        );
        assert!(
            !replacement_bot_available(&pool).await.unwrap(),
            "can_replace_humans on a DISABLED bot must not count"
        );

        // Enabled AND flagged -> available.
        sqlx::query("UPDATE bots SET can_replace_humans = true WHERE name = 'easy'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(replacement_bot_available(&pool).await.unwrap());

        // Ordering is display_order, not name or insertion order.
        sqlx::query("UPDATE bots SET display_order = 99 WHERE name = 'easy'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            find_enabled_bots(&pool).await.unwrap(),
            vec![
                "medium".to_string(),
                "hard".to_string(),
                "easy".to_string()
            ],
            "ordered by display_order"
        );
    }
```

- [ ] **Test 8 — user getters and setters round trip.** Covers `get_user`, `get_user_by_email`, `get_user_name`, `find_user_id_by_name`, `set_user_name`, `get_user_pref_colors`, `set_user_pref_colors`.

```rust
    /// ws F35: seven untested single-statement user getters/setters, batched
    /// into one round-trip test per the coverage cut rule.
    #[sqlx::test]
    async fn user_getters_and_setters_round_trip(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let email = format!("alice-{}@example.com", Uuid::new_v4());
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)")
            .bind(user.id)
            .bind(&email)
            .execute(&pool)
            .await
            .unwrap();

        // get_user / get_user_by_email
        assert_eq!(get_user(&pool, user.id).await.unwrap().unwrap().id, user.id);
        assert!(get_user(&pool, Uuid::new_v4()).await.unwrap().is_none());
        assert_eq!(
            get_user_by_email(&pool, &email).await.unwrap().unwrap().id,
            user.id
        );
        assert!(
            get_user_by_email(&pool, "nobody@example.com").await.unwrap().is_none()
        );

        // get_user_name / find_user_id_by_name (case-insensitive)
        assert_eq!(get_user_name(&pool, user.id).await.unwrap(), "alice");
        assert_eq!(find_user_id_by_name(&pool, "ALICE").await.unwrap(), Some(user.id));
        assert_eq!(find_user_id_by_name(&pool, "nobody").await.unwrap(), None);

        // set_user_name: success, then a case-insensitive conflict -> Ok(false)
        assert!(set_user_name(&pool, user.id, "alice2").await.unwrap());
        assert_eq!(get_user_name(&pool, user.id).await.unwrap(), "alice2");
        let other = make_user(&pool, "bob").await;
        assert!(
            !set_user_name(&pool, other.id, "ALICE2").await.unwrap(),
            "a case-insensitive name clash must be Ok(false), not an error"
        );
        assert_eq!(get_user_name(&pool, other.id).await.unwrap(), "bob");

        // pref colors: empty by default, round-trip, legacy names normalized
        assert!(get_user_pref_colors(&pool, user.id).await.unwrap().is_empty());
        set_user_pref_colors(&pool, user.id, &["Green".to_string(), "Amber".to_string()])
            .await
            .unwrap();
        assert_eq!(
            get_user_pref_colors(&pool, user.id).await.unwrap(),
            vec!["Green".to_string(), "Orange".to_string()],
            "stored legacy 'Amber' must read back normalized to 'Orange'"
        );
        // Unknown user -> empty, not an error.
        assert!(get_user_pref_colors(&pool, Uuid::new_v4()).await.unwrap().is_empty());
    }
```

  Verified so you do not have to guess: `crate::theme::PLAYER_COLOR_NAMES` (`theme.rs:65-67`) is `["Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink"]`, so `"Green"` passes through `normalize_pref_color` unchanged; `normalize_pref_color` (`db.rs:891-903`) maps `"Amber" -> "Orange"` and `"BlueGrey" -> "Cyan"` case-insensitively. `set_user_name` (`db.rs:2798-2809`) returns `Ok(false)` specifically on SQLSTATE `23505`, which `users_name_lower_key` raises for a case-insensitive clash. `get_user_pref_colors` (`db.rs:2815-2823`) returns `Vec::default()` (empty) for an unknown user id rather than erroring.

- [ ] **Test 9 — game and game-version lookups.** Covers `find_game`, `find_game_version`, `find_latest_non_deprecated_game_version`, `find_game_version_rules`, `find_game_version_render_meta`.

```rust
    /// ws F35: five untested lookups, batched. The only one with real logic is
    /// `find_latest_non_deprecated_game_version`, which must skip deprecated
    /// rows.
    #[sqlx::test]
    async fn game_and_version_lookups(pool: PgPool) {
        let (game_type_id, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let game = make_game_with_players(&pool, gv, a.id, &[], 1, &[0]).await;

        // find_game
        let found = find_game(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(found.id, game.id);
        assert_eq!(found.game_version_id, gv);
        assert!(find_game(&pool, Uuid::new_v4()).await.unwrap().is_none());

        // find_game_version
        let version = find_game_version(&pool, gv).await.unwrap().unwrap();
        assert_eq!(version.id, gv);
        assert_eq!(version.game_type_id, game_type_id);
        assert!(!version.is_deprecated);
        assert!(find_game_version(&pool, Uuid::new_v4()).await.unwrap().is_none());

        // rules default to '' (migrations/004) and round-trip
        assert_eq!(find_game_version_rules(&pool, gv).await.unwrap(), Some(String::new()));
        sqlx::query("UPDATE game_versions SET rules = $1 WHERE id = $2")
            .bind("how to play")
            .bind(gv)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            find_game_version_rules(&pool, gv).await.unwrap(),
            Some("how to play".to_string())
        );
        assert!(find_game_version_rules(&pool, Uuid::new_v4()).await.unwrap().is_none());

        // render meta
        let (uri, name, iface) = find_game_version_render_meta(&pool, gv).await.unwrap().unwrap();
        assert_eq!(uri, "http://localhost:0/mock");
        assert_eq!(name, "1.0.0");
        assert!(iface >= 1, "interface_version should have a sane default, got {iface}");
        assert!(
            find_game_version_render_meta(&pool, Uuid::new_v4()).await.unwrap().is_none()
        );

        // latest non-deprecated must skip a deprecated newer row
        let newer: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '2.0.0', 'http://localhost:0/mock2', true, true) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let latest = find_latest_non_deprecated_game_version(&pool, game_type_id)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(latest.id, newer, "a deprecated version must never be chosen");
        assert_eq!(latest.id, gv);
        assert!(
            find_latest_non_deprecated_game_version(&pool, Uuid::new_v4())
                .await
                .unwrap()
                .is_none()
        );
    }
```

  Verified so you do not have to guess: `find_latest_non_deprecated_game_version` (live :223-241) is `... WHERE game_type_id = $1 AND is_deprecated = false ORDER BY created_at DESC LIMIT 1`, so the newer-but-deprecated `'2.0.0'` row is excluded by the `WHERE`, not merely out-ordered — `gv` is the only candidate and the assertion is deterministic. `find_game_version_rules` (:262-271) selects `rules`, which is `TEXT NOT NULL DEFAULT ''` (`migrations/004_game_version_rules.sql:2`), so the default really is `Some(String::new())` and never `None` for an existing row. `find_game_version_render_meta` (:274-283) selects `uri, name, interface_version`, and `interface_version` is `INTEGER NOT NULL DEFAULT 1` (`migrations/013_bot_efficacy.sql:38`). `find_game_version` returns `crate::models::game::GameVersion` with fields `id, created_at, updated_at, game_type_id, name, uri, is_public, is_deprecated`; `find_game` returns `crate::models::game::Game` with `id, created_at, updated_at, game_version_id, is_finished, finished_at, game_state, chat_id, restarted_game_id` — the field names the test uses all exist.

- [ ] **Test 10 — friend-request helpers.** Covers `count_incoming_friend_requests`, `get_pending_request_source`, `has_block_conn`.

```rust
    /// ws F35: three untested friend/block helpers, batched.
    #[sqlx::test]
    async fn friend_request_helpers(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let x = make_user(&pool, "requesterx").await;
        let y = make_user(&pool, "requestery").await;

        assert_eq!(count_incoming_friend_requests(&pool, me.id).await.unwrap(), 0);
        send_friend_request(&pool, x.id, me.id).await.unwrap();
        send_friend_request(&pool, y.id, me.id).await.unwrap();
        assert_eq!(count_incoming_friend_requests(&pool, me.id).await.unwrap(), 2);

        let incoming = list_incoming_friend_requests(&pool, me.id).await.unwrap();
        let (req_id, _, _) = incoming[0];
        let source = get_pending_request_source(&pool, req_id, me.id).await.unwrap();
        assert!(source == Some(x.id) || source == Some(y.id));
        assert_eq!(
            get_pending_request_source(&pool, req_id, x.id).await.unwrap(),
            None,
            "only the TARGET of the request may resolve its source"
        );
        assert_eq!(
            get_pending_request_source(&pool, Uuid::new_v4(), me.id).await.unwrap(),
            None
        );

        // Once responded, it is no longer pending and drops out of both.
        respond_to_friend_request(&pool, req_id, me.id, true).await.unwrap();
        assert_eq!(count_incoming_friend_requests(&pool, me.id).await.unwrap(), 1);
        assert_eq!(get_pending_request_source(&pool, req_id, me.id).await.unwrap(), None);

        // has_block_conn must agree with has_block, and is directional.
        block_user(&pool, me.id, x.id).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        assert!(has_block_conn(&mut conn, me.id, x.id).await.unwrap());
        assert!(!has_block_conn(&mut conn, x.id, me.id).await.unwrap());
        assert_eq!(
            has_block_conn(&mut conn, me.id, x.id).await.unwrap(),
            has_block(&pool, me.id, x.id).await.unwrap()
        );
    }
```

- [ ] **Test 11 — the coverage inventory guard.** This is the test that keeps ws F35 from silently reopening.

```rust
    /// ws F35 guard: the functions listed below were among the 27 public db.rs
    /// functions with ZERO test references at review time. 25 of them are now
    /// covered by name in this module; the two exclusions are documented in the
    /// WP-41 spec (`create_pool` needs a real DATABASE_URL outside the
    /// per-test database; `create_game_with_users_tx` is the body of the
    /// `create_game_with_users` wrapper, exercised by every fixture game).
    ///
    /// This test is a *reminder*, not a mechanism: it re-asserts the cheapest
    /// invariant of each newly covered function so that deleting one of the
    /// tests above still leaves a failing signal here.
    #[sqlx::test]
    async fn ws_f35_previously_untested_functions_are_reachable(pool: PgPool) {
        let (game_type_id, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;
        let mut conn = pool.acquire().await.unwrap();

        assert_eq!(count_incoming_friend_requests(&pool, a.id).await.unwrap(), 0);
        assert!(find_active_turn_games(&pool, a.id, 5).await.unwrap().len() == 1);
        // NOT is_empty(): migrations/013_bot_efficacy.sql:41-44 seeds three
        // enabled bots into every test database.
        assert_eq!(find_enabled_bots(&pool).await.unwrap().len(), 3);
        assert!(find_game(&pool, game.id).await.unwrap().is_some());
        assert!(find_game_version(&pool, gv).await.unwrap().is_some());
        assert!(find_game_version_render_meta(&pool, gv).await.unwrap().is_some());
        assert!(find_game_version_rules(&pool, gv).await.unwrap().is_some());
        assert!(
            find_latest_non_deprecated_game_version(&pool, game_type_id)
                .await
                .unwrap()
                .is_some()
        );
        assert!(find_open_restart_proposal(&pool, game.id).await.unwrap().is_none());
        let mut tx = pool.begin().await.unwrap();
        assert!(find_open_restart_proposal_tx(&mut tx, game.id).await.unwrap().is_none());
        tx.rollback().await.unwrap();
        assert_eq!(find_user_id_by_name(&pool, "alice").await.unwrap(), Some(a.id));
        assert!(validate_username(&generate_unique_username(&mut conn).await.unwrap()));
        assert!(get_pending_request_source(&pool, Uuid::new_v4(), a.id).await.unwrap().is_none());
        assert!(get_user(&pool, a.id).await.unwrap().is_some());
        assert!(get_user_by_email(&pool, "nobody@example.com").await.unwrap().is_none());
        assert_eq!(get_user_name(&pool, a.id).await.unwrap(), "alice");
        assert!(get_user_pref_colors(&pool, a.id).await.unwrap().is_empty());
        assert!(!has_block_conn(&mut conn, a.id, b.id).await.unwrap());
        assert!(!is_user_admin(&pool, a.id).await.unwrap());
        mark_game_read(&pool, game.id, a.id).await.unwrap();
        assert!(!replacement_bot_available(&pool).await.unwrap());
        assert!(set_user_name(&pool, a.id, "alice_renamed").await.unwrap());
        set_user_pref_colors(&pool, a.id, &[]).await.unwrap();
        assert!(!should_hide_add_friend(&pool, a.id, b.id).await.unwrap());

        let mut tx = pool.begin().await.unwrap();
        insert_game_logs_tx(&mut tx, game.id, vec![]).await.unwrap();
        tx.commit().await.unwrap();
    }
```

- [ ] Re-run the inventory to confirm the gap closed. From `/home/beefsack/Development/brdgme/rust/web/src`:

```bash
for f in count_incoming_friend_requests find_active_turn_games find_enabled_bots \
  find_game find_game_version find_game_version_render_meta find_game_version_rules \
  find_latest_non_deprecated_game_version find_open_restart_proposal \
  find_open_restart_proposal_tx find_user_id_by_name generate_unique_username \
  get_pending_request_source get_user get_user_by_email get_user_name \
  get_user_pref_colors has_block_conn insert_game_logs_tx is_user_admin \
  mark_game_read replacement_bot_available set_user_name set_user_pref_colors \
  should_hide_add_friend; do
  n=$(awk "NR>$(grep -n '^mod tests {' db.rs | cut -d: -f1)" db.rs | grep -c "\b$f(")
  echo "$n $f"
done | sort -n
```

  Expected: every one of the 25 counts `>= 1`. (The `mod tests {` line is live :3140 before this package's edits and moves as earlier tasks collapse strings, hence the inline `grep` rather than a hard-coded number.)

**Test plan summary — where each test goes and what it proves:**

| Test | Functions covered | Key cases |
|---|---|---|
| `find_active_turn_games_orders_oldest_turn_first_and_caps` | `find_active_turn_games` | 3 games with `is_turn_at` 01-03/01-01/01-02 → returned in 01,02,03 order; not-my-turn game excluded; finished game excluded; returned player id is the caller's; `cap = 2` → 2 rows; other user → empty |
| `generate_unique_username_is_valid_and_unclaimed` | `generate_unique_username` (+`find_user_id_by_name`, `validate_username`) | result validates D2 and is unused; claim it → second call differs case-insensitively and inserts cleanly |
| `insert_game_logs_tx_writes_logs_and_targets` | `insert_game_logs_tx` | 2 logs, one public/no targets, one private with `to = [0,1,9]` → 2 log rows with exact body/is_public/logged_at; 2 target rows; position 9 dropped |
| `mark_game_read_marks_only_the_caller_in_that_game` | `mark_game_read` | only (g1, alice) flips; g2 and bob untouched; unknown user → no-op `Ok` |
| `should_hide_add_friend_covers_every_row_state` | `should_hide_add_friend` | stranger → false; outgoing pending/declined/accepted → true; incoming pending → false |
| `find_open_restart_proposal_finds_earliest_open_only` | `find_open_restart_proposal`, `find_open_restart_proposal_tx` | none → None; cancelled only → None; two open → earliest; `_tx` agrees; unrelated game → None |
| `bot_lookups_respect_enabled_and_can_replace_humans` | `find_enabled_bots`, `replacement_bot_available` | seeded baseline `["easy","medium","hard"]`/false (migration 013 seeds three enabled bots); a disabled `'offbot'` excluded and its `can_replace_humans` ignored; enabled+flagged → true; `display_order` reshuffle changes the order |
| `user_getters_and_setters_round_trip` | `get_user`, `get_user_by_email`, `get_user_name`, `find_user_id_by_name`, `set_user_name`, `get_user_pref_colors`, `set_user_pref_colors` | hit/miss for each getter; case-insensitive name lookup; rename → `Ok(true)`; case-clash → `Ok(false)` and no write; legacy "Amber" reads back as "Orange"; unknown user → empty vec |
| `game_and_version_lookups` | `find_game`, `find_game_version`, `find_game_version_rules`, `find_game_version_render_meta`, `find_latest_non_deprecated_game_version` | hit/miss for each; rules default `""` then round-trip; render meta triple; deprecated newer version never chosen |
| `friend_request_helpers` | `count_incoming_friend_requests`, `get_pending_request_source`, `has_block_conn` | 0 → 2 → 1 after responding; only the target resolves the source; unknown id → None; responded → None; `has_block_conn` directional and equal to `has_block` |
| `ws_f35_previously_untested_functions_are_reachable` | all 25 (the 24 here plus `is_user_admin`) | one cheap invariant each; the guard against silent regression of the coverage inventory |

**Command the future implementer runs:** `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr db::tests` (with `scripts/rust-test.sh`'s containers up, or run the whole script).

**Verification checkpoint:**

- [ ] All 11 new tests pass.
- [ ] `cargo test -p web --features ssr db::tests` — the whole db test module passes, with exactly the two sanctioned edits (Tasks 3 and 7) differing from the pre-package suite.
- [ ] The inventory loop above prints `>= 1` for all 25 names.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `test(db): cover 24 previously untested public DB functions (ws F35)`

---

## Final gate (before the last commit — mandatory)

- [ ] Regenerate the sqlx offline cache. SQL text changed in `sqlx::query!` macro calls in Tasks 1 and 3 (`concede_game`, `end_game`, `delete_game`, `mark_game_read`, `undo_game`, `apply_rating_changes`, `update_game_command_success`), so `.sqlx/` is stale and `cargo sqlx prepare --check` will fail until it is regenerated. Run the scratch-database flow from Global Constraints, then:
      `SQLX_OFFLINE=true cargo check -p web --features ssr`
      `(cd /home/beefsack/Development/brdgme/rust/web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`
      Commit the `.sqlx/` diff **with** the code, in the same commit or an immediately following `chore(db): regenerate .sqlx offline cache` commit.
- [ ] `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — the full CI sequence (migrations, fmt, clippy×2, sqlx prepare check, tests×2) exits 0.
- [ ] `git diff --stat origin/master...HEAD` touches **only** `rust/web/src/db.rs` and `rust/web/.sqlx/`. If any other file appears, you absorbed something from the Non-Goals list — revert it.
- [ ] Re-read the disposition table and confirm each row's stated action landed, and that nothing marked OVERTURNED / SKIPPED / FENCED was implemented anyway. Specifically confirm: `friends.rs` untouched; `suggestions_exclude_blocked_and_self` unrenamed; `is_turn_at`'s assignment unchanged; `insert_game_logs_tx` unbatched; db.rs unsplit; `undo_game`'s rating fields untouched; `apply_rating_changes`' `if change == 0 { continue; }` skips untouched; `delete_game`'s two `game_proposals` `updated_at` sets intact.

---

## Cross-package / newly discovered

None of the following is in WP-41's scope. Do not fix them here.

1. **`($1 || ' seconds')::interval` survives at four other sites.** Task 6 removes the db.rs instance, so the house idiom becomes inconsistent: `proposals.rs:725`, `proposals.rs:755`, `proposals.rs:819` and `email/sweep.rs:65` still format an integer into text and re-parse it as an interval. Same analysis as ws F47 — **not injectable** (bound parameter, `i64::to_string()`), purely a typing wart. **Routed to WP-44 (proposals.rs) and WP-46 (email/sweep.rs, and proposals.rs where those packages overlap)** as a rider on work already in those files. Evidence: `grep -rn "seconds')::interval" /home/beefsack/Development/brdgme/rust/web/src`.

2. **`find_active_turn_games`' `NULLS LAST` is dead SQL.** db.rs:3112 orders `BY gp.is_turn_at ASC NULLS LAST`, but `game_players.is_turn_at` is declared `timestamp without time zone NOT NULL` (`migrations/001_initial_schema.sql:193`) and no later migration relaxes it, so the clause can never take effect. Harmless and *not changed by this package* (removing it is churn on a query WP-52 may rewrite). ws F35's recommendation asked for a test of "NULLS LAST" specifically — that is untestable, and Task 11's test says so in a comment rather than faking it. **Routed to WP-52** (stats/query performance pass) as a one-line cleanup if it rewrites that query; otherwise leave it.

3. **`concede_game`'s 2-player assumption is a `debug_assert!` only.** db.rs:1315 `debug_assert!(players.len() == 2, "concede_game assumes exactly 2 players")`, and the place-assignment loop at :1316-1329 gives place 1 to every non-conceder — so a release build silently mis-places a 3+ player game, and `apply_rating_changes` then rates that wrong outcome. ws F35's recommendation bundles a fix, but the function is **WP-40's** (`undo/concede TOCTOU + ratings integrity`, BLOCKED-ON-DECISION D-3) and the correct behaviour depends on D-3's ruling. **Routed to WP-40.** Not tested here — writing a test would mean pinning either the current wrong behaviour or a fix this package is not allowed to make.

4. **`db.rs` module split (ws F42) is deferred, not dropped.** Rationale in the disposition table. It should be scheduled **after** WP-35, WP-40, WP-45, WP-47, WP-49, WP-50, WP-52, WP-53 and WP-59 have landed their db.rs edits, as its own package, and it should be mechanical-only (move items, add `pub(crate)` where needed, split the test module alongside its subject) with no behaviour change in the same commit. **Route: add to `docs/BACKLOG.md` / a new work package; do not fold into any decision-blocked package.**

5. **Five "pure predicate" helpers are `#[cfg(feature = "ssr")]`-gated even though they are pure and their doc comments present them as shared logic.** `active_within_window` (db.rs:2001-2002), `can_remove_email` (:2909-2910), `can_switch_to_email` (:2916-2917), `is_expired_unverified` (:2923-2924) and `cap_digest` (:2938-2939) all carry the gate, unlike `validate_username` (:849), which is genuinely ungated and *is* called from the client-side settings form. Every current caller of the gated five is server-side (`auth/server.rs:653,887,1363`, `game/import.rs:178`, `email/commands.rs:458,483`, plus db.rs itself), so **nothing is broken today** and this package does not touch them. It is recorded because an earlier draft of this spec asserted the opposite and any future work that wants to reuse these predicates in a WASM component will have to remove the gates. **Routed to WP-54 (frontend UX) as a note, to be actioned only if a client-side caller actually appears.** Evidence: `grep -n -B1 "^pub fn " /home/beefsack/Development/brdgme/rust/web/src/db.rs`.

6. **`friends` carries two overlapping unique indexes.** `010_friends.sql:5-6` creates `friends_source_target_key` on `(source_user_id, target_user_id)` and `:7-9` creates `friends_pair_key` on `(LEAST(...), GREATEST(...))`. The pair index strictly subsumes the directional one for uniqueness purposes; the directional one may still be earning its keep as a lookup index. Not investigated further and **not changed** (it would need a migration, which this package forbids). **Routed to `docs/BACKLOG.md` as a schema-hygiene note; needs a user decision before anyone drops an index.**
