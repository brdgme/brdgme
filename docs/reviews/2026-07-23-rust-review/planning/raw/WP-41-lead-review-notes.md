# WP-41 spec adversarial review - Worker notes (2026-07-25)

Target: `planning/specs/WP-41-db-quality-pass.md`. Method: read-only verification against
live source at `/home/beefsack/Development/brdgme` plus the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`. No cargo, no git mutation, no
edits under `rust/`.

**Verdict: ACCEPT-AFTER-MY-REPAIRS.** The spec's reasoning is sound - all five OVERTURNED /
narrowed justifications hold up independently - but it shipped one test that cannot pass, one
false statement that would have been written into `db.rs` as a doc comment, two wrong
migration line ranges that would also have been written into `db.rs`, a wrong caller
inventory, an off-by-one function count, and about a dozen off-by-one-to-off-by-three line
anchors.

## 1. Citation audit

**64 citations checked. 21 wrong.** Nothing was wrong in a way that invalidates a fix's
*intent*; the failures are (a) one factual claim about `#[cfg]` gating, (b) migration trigger
line ranges, (c) a fixture assumption, (d) counts, (e) line drift of 1-3.

### Confirmed exactly right

| Claim | Live evidence | Result |
|---|---|---|
| db.rs is 6877 lines | `wc -l` = 6877 | CONFIRMED |
| snapshot 6380 -> live 6877, 13 hunks | `wc -l` snapshot = 6380; `diff -u \| grep -c '^@@'` = 13 | CONFIRMED (the "+503/-6" was a naive grep incl. header lines; content is +502/-5, net +497) |
| F36: 25 sweep sites on trigger-maintained tables | grep for `updated_at = NOW()\|updated_at = timezone` below :3140 returns **27** lines; minus :1487/:1493 = **25** | CONFIRMED |
| F36: exclude live :1487 / :1493 | both are `UPDATE game_proposals ...` | CONFIRMED |
| 14 tables carry `update_updated_at` | 14 `CREATE OR REPLACE TRIGGER update_*_updated_at` at 001:392-446 | CONFIRMED |
| No later migration adds a trigger | `grep -rn "CREATE TRIGGER\|CREATE OR REPLACE TRIGGER" migrations/ \| grep -v 001_initial` is empty | CONFIRMED |
| `friends_pair_key` at 010:7-9 | LEAST/GREATEST expression index | CONFIRMED |
| `friends_check` at 001:114 | `CHECK ((target_user_id <> source_user_id))` | CONFIRMED |
| `users_name_lower_key` at 009:41 | `CREATE UNIQUE INDEX users_name_lower_key ON public.users (lower(name))` | CONFIRMED |
| `game_players.is_turn_at` NOT NULL at 001:193 | `is_turn_at timestamp without time zone NOT NULL` | CONFIRMED |
| 013:10 / 013:20 `updated_at`, no trigger | bots / llm_providers | CONFIRMED |
| 015:8 / 015:22 `updated_at`, no trigger | game_proposals / game_proposal_players | CONFIRMED |
| F37 anchor :1891 | `is_finished = $2 ... COALESCE($3, finished_at) ... AND updated_at = $5` | CONFIRMED |
| F37 dangling comment :4858-4862 | ends `// preserving it - this differs from the plan's phrasing, see report.` | CONFIRMED (exact) |
| F44 anchor :1921 | `let is_turn_at = if is_turn { now } else { p_is_turn_at };` | CONFIRMED |
| `email/sweep.rs:65` interval, :60-68 predicate | `AND gp.is_turn_at < NOW() - ($1 \|\| ' seconds')::interval` on :65 | CONFIRMED |
| `friends.rs:170-172` self-friend rejection | `if target == user.id { return Err(ServerFnError::new("You cannot friend yourself")); }` | CONFIRMED |
| `index.rs:52` passes scan_limit 10 per friend | inside `for (friend_id, friend_name) in friends` at :51 | CONFIRMED |
| `error.rs:7` internal signature | `pub fn internal<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> ServerFnError` | CONFIRMED |
| `game/export.rs:195-202` match arm | `Ok(true)/Ok(false)/Err(e) => tracing::error!` | CONFIRMED |
| `game/server_fns.rs:369` reads only `rating` | `rating: p.game_type_user.rating` | CONFIRMED |
| No caller checks the nil id | `grep -rn "Uuid::nil\|is_nil" web/src` -> only db.rs:100, :104 | CONFIRMED |
| F43 synthetic arm :99-108 | `id: Uuid::nil()`, rating/peak 1200, `last_game_finished_at: None` | CONFIRMED |
| F51(1) overturn | `suggestions_exclude_blocked_and_self` :4192-4210 builds the fixture with `creator_id = me.id`; a broken self-exclusion fails the `is_empty()` at :4209 | CONFIRMED |
| `self_request_rejected_by_db_check` :3492-3496, Err assert on :3495 | exact | CONFIRMED |
| `count_rows` :6529-6534 | `format!`-built COUNT(*) | CONFIRMED |
| `// --- Unit B2 public index game selection ---` at :3813 | insertion point for Task 10(2) | CONFIRMED |
| WP-41 scope = 16 findings F35-F51, path web/src/db.rs | work-packages.md:329-335 | CONFIRMED |
| Snapshot mapping :1357/:1363 -> :1487/:1493 and :3317 -> :3495 | work-packages.md NOTE vs live | CONFIRMED |
| Postgres 18 on 15432, NATS on 14222 | scripts/rust-test.sh:27,31,60,61 | CONFIRMED |
| DEV.md scratch-DB sqlx flow, 130 `.sqlx` files, toolchain 1.97.0 | as quoted | CONFIRMED |
| CODING.md requires tests for db.rs changes | docs/CODING.md:560-563 | CONFIRMED |

### Refuted / wrong

| # | Spec claim | Live evidence | Verdict |
|---|---|---|---|
| W1 | `can_remove_email`, `can_switch_to_email`, `is_expired_unverified`, `cap_digest`, `active_within_window` are "deliberately **ungated** so the WASM client can share them" | all five carry `#[cfg(feature = "ssr")]` at db.rs:2909, :2916, :2923, :2938, :2001. Only `validate_username` (:849) is ungated | **REFUTED.** Was going to be written verbatim into db.rs's module header |
| W2 | `update_finished_at` trigger at 001:440-444 | it is at **001:448-452**; 440-444 is `update_game_logs_updated_at` (self-contradictory with the spec's own correct "001:392-446 covers the 14") | **REFUTED.** Also destined for db.rs comments (Task 1 and Task 3) |
| W3 | `update_is_turn_at` trigger at 001:446-450 | it is at **001:454-458** | **REFUTED.** Destined for the Task 4 in-source comment |
| W4 | (omission) no mention of `update_last_turn_at` | third conditional trigger at **001:460-464**, `WHEN old.is_turn = true AND new.is_turn = false`, on the same table and interacting with the same UPDATE | **GAP** |
| W5 | `update_updated_at()` at 001:25-33 | 001:**25-32** | minor, but it goes into source |
| W6 | Task 11 Test 7 asserts `find_enabled_bots(...).is_empty()` and inserts bots named `'easy'`/`'hard'` | `migrations/013_bot_efficacy.sql:41-44` **seeds** `('easy',0), ('medium',1), ('hard',2)`, all `enabled = true`, and `bots.name` is `NOT NULL UNIQUE` | **REFUTED - test cannot pass.** First assert fails; the INSERT then raises 23505 |
| W7 | Task 11 Test 11 asserts `find_enabled_bots(...).is_empty()` | same seed | **REFUTED - test cannot pass** |
| W8 | "**26** public fns with zero test references"; "25 remain for this task"; "24 of the 26" | the spec's own list has **27** names, and my independent derivation returns exactly those 27 | **REFUTED (count only; the list is right)** |
| W9 | "all **19** call sites [of `is_user_admin`] either `.map_err(internal(...))` or `match` on `Err(e)`" | **20** call sites outside db.rs. 18 `.map_err`, 1 `match`, and 1 **`.await.unwrap()`** at `admin.rs:2201` (inside admin.rs's `mod tests`, module opens :2177) | **REFUTED.** Still compiles (`anyhow::Error: Debug`), but the spec's own "STOP and report if any hit does something else" gate would have stalled the implementer |
| W10 | F49 anchor :976; `pos` uses at :977, :981 | `for (pos, pref) in rem_prefs.clone() {` is on :**977**; `contains_key(&pos)` on :**978**; `insert(pos, ...)` on :**983** | wrong anchors (the quoted replacement block itself is correct Rust) |
| W11 | F50 loop header :1799-1805, body :1806-1817 | header :**1802-1806**, inner `for` :1807, body :1808-1818 | wrong anchors |
| W12 | `turn_reminder_sent_at = NULL` at :1936 | :**1934** | wrong |
| W13 | F47 interval at :3130-3132; body :3129-3137 | interval on a single line :**3131**; body :**3128-3136** | wrong |
| W14 | F43 doc insertion "above the `#[cfg]` at live :55" | `#[cfg]` is at :**56**, with `#[allow(clippy::too_many_arguments)]` at :58 between it and the `fn` at :59 | wrong |
| W15 | F43's existing test "at live :4590-4620" | `find_game_extended_missing_game_type_user_defaults_to_1200`, attr :**4593**, fn :4594, `}` :4623 | wrong, and unnamed |
| W16 | Task 3's existing `assert_eq!` at :4884-4888 | :**4885-4889**; test `}` at :4890 | wrong |
| W17 | Task 5 test plan: "the `undo_game` tests that assert `finished_at.is_none()` (:5494) and `.is_some()` (:5540)" | :5494 is in an undo test; :**5540 is in a `concede_game` test** (`concede_game(...)` on :5534) | mislabelled |
| W18 | Task 6's existing test "at live :6770-6820" | `expiry_cleanup_deletes_only_expired_unverified`, attr :**6771**, fn :6772 | wrong, and unnamed |
| W19 | `friend_row_state` "used at live :3465" | **defined** at :3364-3375 | misleading |
| W20 | `NULLS LAST` at db.rs:3111 | :**3112** | wrong |
| W21 | F42: "db.rs is on the declared path list of **eight** other packages... **five** of which are decision-blocked" | **nine** (WP-59 also lists `web/src/db.rs`), of which **six** are decision-blocked | wrong |

Also minor and corrected: `mod tests {` is at :3140 (attr :3139), not :3139; the drift
narrative's item 3 claimed `concede_game`'s UPDATE changed - no diff hunk touches
`concede_game`, it only shifted +7; the narrative claimed 5 test hunks, there is 1.

## 2. Re-derivation of the OVERTURNED / narrowed calls

All five hold. Independently derived:

- **F44 (is_turn_at reset) - OVERTURN HOLDS.** :1921 re-stamps on every command; the same
  UPDATE nulls `turn_reminder_sent_at` on :1934; `email/sweep.rs:62-68` gates on
  `is_turn = true AND is_eliminated = false AND turn_reminder_sent_at IS NULL AND is_turn_at <
  NOW() - threshold AND game_bot_id IS NULL AND u.reminder_emails_enabled`. The two resets are
  in one statement, so "last turn activity" is the coherent reading. `find_active_turn_games`
  (:3101) orders `is_turn_at ASC` for the digest. Changing the assignment would nag a player
  who just acted. Confirmed.
- **F51(1) (test name over-promises) - OVERTURN HOLDS.** See table.
- **F37's clear-`finished_at` option - REJECTION HOLDS.** `undo_game` :1547 writes
  `is_finished = $2` **and** `finished_at = NULL` in one statement, so it is the un-finish
  path; clearing on the command path would erase a real finish. Also verified the sticky form
  does not disturb `update_finished_at`, which fires only false->true.
- **F39's `ON CONFLICT` option - REJECTION HOLDS.** `friends_pair_key` is a two-expression
  index (010:7-9), so `ON CONFLICT` would need matching inference expressions, and
  `DO NOTHING` would swallow the auto-accept in the `Some(r)` arm at :2089-2099. The advisory
  lock is taken as the transaction's first statement and is the only lock the function takes,
  so the no-deadlock argument holds. `hashtext(text)` and `pg_advisory_xact_lock(int4,int4)`
  both exist in Postgres; `LEAST`/`GREATEST` on uuid works.
- **F46's retry-on-23505 - REJECTION HOLDS.** All four callers pass a connection inside an
  open transaction; a 23505 aborts the transaction, so a retry needs SAVEPOINTs in four
  modules.
- **F43's `Option<GameTypeUser>` - NARROWING HOLDS.** `grep -rn "Uuid::nil\|is_nil" web/src`
  returns only db.rs:100 and :104; the only field read off the struct outside db.rs is
  `rating` (`game/server_fns.rs:369`); `GameTypeUser` lives in `crate::models::game`, WP-53's
  path.

## 3. Repairs applied to the spec

Structure preserved; no wholesale rewrite. 45 edits, grouped:

1. **Architecture paragraph** - corrected the `#[cfg]` gating claim (W1) and pinned
   `mod tests` at :3140.
2. **DB-machinery bullets** - corrected 001:25-32, 001:448-452, 001:454-458; added
   `update_last_turn_at` 001:460-464 (W2-W5); added reproduce-it grep commands.
3. **Snapshot-drift section** - rewrote the hunk narrative against the actual 13 hunk headers,
   corrected the diff counts, corrected 8 entries of the finding->live mapping table.
4. **Disposition table** - F35 26->27 and restated the arithmetic; F37/F39/F40/F41/F49/F50/F51
   anchors corrected; F45 rewritten with the true 20-site inventory; F42's package count
   8->9 and blocked count 5->6.
5. **Task 1** - corrected the module-header comment that was about to put W1/W2/W3/W5 into
   `db.rs`; added a "do not paraphrase these trigger line numbers" warning; annotated all 25
   sweep entries with the exact `updated_at` line plus the enclosing literal range (items
   2, 3, 5, 9, 10, 12, 14, 15 were off); corrected the `delete_game` comment location to
   :1484-1485 and made the edit instruction concrete ("replace those two lines with this
   four-line block"); stated the pre-task grep count (27) and why 3200 is a safe awk bound.
6. **Task 2** - replaced "19 call sites" with the enumerated 20, called out `admin.rs:2201`,
   and rewrote the STOP-and-report gate so it names the only two shapes that would actually
   break compilation (`?` in an `sqlx::Result` fn, matching an `sqlx::Error` variant).
7. **Task 3** - corrected the assert anchor to :4885-4889, named the test's structure by line,
   corrected the trigger citation, noted that `update_finished_at` *does* overwrite
   `finished_at` on the genuine first finish (pre-existing, unchanged), and fixed the
   mislabelled :5540 concede test (W17).
8. **Task 4** - corrected :1934 and 001:454-458 inside the comment that lands in source;
   quoted the sweep predicate in full with per-line citations; named the pinning test
   `update_game_command_success_mid_turn_keeps_last_turn_at` (:4759-4808) instead of a bare
   line range.
9. **Task 5** - corrected F49 anchors (:977/:978/:983) and added the `LocPref` type reasoning
   that makes `contains_key(pos)` / `insert(*pos, ...)` provably correct; corrected F50
   anchors (:1802-1807 header, :1808-1818 body) and recorded that `rated_players` is a
   `Vec` so the slice form compiles; corrected the F43 insertion point to above :56 and
   warned about the intervening `#[allow]`; extended the F43 doc text to mention
   `created_at`/`updated_at`/`user_id` (the sentinel's `user_id` can also be nil, which the
   original text omitted); named F43's existing test and corrected its range to :4593-4623.
10. **Task 6** - corrected the body range to :3128-3136 and the interval to :3131; named the
    existing test `expiry_cleanup_deletes_only_expired_unverified` (:6771+); replaced the
    "if `UserEmailRow`'s field is not named `email`" hedge with the verified struct
    definition (:2895-2901).
11. **Task 7** - corrected the doc-comment range to :2049-2053, the read/insert/match arm
    ranges, and made explicit that the advisory lock must precede the blocked-source check
    at :2057-2066; replaced "`friend_row_state` used at :3465" with its definition and
    signature; replaced the hand-listed 18 test line numbers with a reproducible grep plus a
    note that `accept_friends` (:3699-3702) also routes through `send_friend_request`.
12. **Task 8** - corrected the per-candidate loop to :2514-2518, the predicate string to
    :2412-2424, and the function range to :2493-2520; added the "INNER join drops bots"
    invariant explicitly; **rewrote the drift-guard test** to give each visibility case its
    own `friend` user instead of `DELETE FROM game_players; DELETE FROM games;` - this
    removes the spec's "if this hits an FK, ... either shape is acceptable" placeholder
    (a quality-bar violation) and is deterministic.
13. **Task 9** - corrected the function range and added the :1263 `if let Some(&player_id)`
    citation that Test 3's "position 9 is dropped" assertion depends on.
14. **Task 10** - corrected the `count_rows` and `suggestions_exclude_blocked_and_self`
    ranges and named the two existing visibility tests by attr/fn/close line.
15. **Task 11** - 26->27 with the arithmetic restated; added a verified **helper signature
    table** (`make_user`, `make_game_type_and_version`, `make_game_with_players`,
    `accept_friends`, `friend_row_state`, `count_rows`) so the implementer never has to
    guess; added an explicit **"the `bots` table is seeded"** fixture warning; **rewrote
    Test 7** so it asserts against the seeded baseline, inserts `'offbot'` instead of a
    colliding `'hard'`, and adds a display_order-reshuffle case; **fixed Test 11's**
    `find_enabled_bots(...).is_empty()` to `.len() == 3`; replaced Test 6's async-closure +
    "if it fights the borrow checker, inline it" hedge with a plain nested `async fn` and
    documented the `&mut tx` deref coercion precedent (:1957); replaced Test 8's "if Green
    is not in PLAYER_COLOR_NAMES" hedge with the verified constant (`theme.rs:65-67`) plus
    `set_user_name`'s 23505 semantics; replaced Test 9's "read the ORDER BY before asserting"
    hedge with the verified query (`WHERE is_deprecated = false ORDER BY created_at DESC`),
    the `rules TEXT NOT NULL DEFAULT ''` fact (004:2) and the `interface_version` default
    (013:38), plus the exact struct field lists for `Game`/`GameVersion`; made the inventory
    loop derive the `mod tests` line instead of hard-coding 3139; corrected the summary table
    rows for Test 7 and Test 11.
16. **Coordination table** - added a row for the `user_emails` functions vs WP-50/WP-59;
    corrected the F50 and `if change == 0` anchors (:1823, :1842).
17. **Cross-package section** - corrected `NULLS LAST` to :3112; added two new items (below).

## 4. Newly discovered defects, routed

- **The five `ssr`-gated "pure predicate" helpers** (`active_within_window`,
  `can_remove_email`, `can_switch_to_email`, `is_expired_unverified`, `cap_digest`). Their doc
  comments read as shared logic but the gate makes them server-only. Nothing is broken today -
  every caller is server-side (`auth/server.rs:653,887,1363`, `game/import.rs:178`,
  `email/commands.rs:458,483`). **Routed to WP-54** as a note, actionable only if a
  client-side caller appears. Added as cross-package item 5.
- **`friends` has two overlapping unique indexes** (`friends_source_target_key` on
  `(source, target)` at 010:5-6, subsumed for uniqueness by `friends_pair_key` at 010:7-9).
  Needs a migration to change, which WP-41 forbids, and dropping an index is a judgement call.
  **Routed to `docs/BACKLOG.md`, flagged as needing a user decision.** Added as cross-package
  item 6.
- The spec's existing cross-package items 1-4 (four surviving interval sites, dead
  `NULLS LAST`, `concede_game`'s `debug_assert!`-only 2-player guard, the F42 split deferral)
  all verified accurate and left in place with corrected line numbers.

## 5. Not closed - needs the Lead or the user

1. **`concede_game`'s release-build mis-placing of 3+ player games** (db.rs:1315 is a
   `debug_assert!`; the loop at :1316-1329 gives place 1 to every non-conceder, and
   `apply_rating_changes` then rates that outcome). The spec correctly routes this to WP-40
   (BLOCKED-ON-DECISION D-3) and correctly refuses to write a test that pins either the wrong
   behaviour or an unauthorised fix. I have not changed that. It remains a live correctness
   hole until D-3 is decided.
2. **Task 7's advisory-lock design is a judgement call I did not second-guess.** The
   reasoning is sound and the escape hatch (STOP if `hashtext` is missing) is reasonable, but
   whether a `pg_advisory_xact_lock` on every friend request is acceptable operationally is a
   call above a spec review - the alternative (SERIALIZABLE + caller retry) is genuinely
   worse, and `ON CONFLICT` is genuinely wrong. Left as written.
3. **Task 11's cut rule excludes `create_pool` and `create_game_with_users_tx`.** Both
   exclusions are well argued. Whether the Lead accepts 25-of-27 as "closing the major" is a
   scope call. Left as written.
4. **I could not execute anything.** Every "would this compile / would this pass" judgement in
   this review is derived from reading source. The two tests I rewrote (Task 8's drift guard,
   Task 11's Test 7) are reasoned, not run. In particular I did **not** verify that
   `ALTER TABLE ... DISABLE TRIGGER` succeeds under `#[sqlx::test]`'s per-test database
   (it should - the migration user owns the tables - but it is unverified), nor that
   `hashtext` is callable on the harness's Postgres 18.

## 6. Compliance

- No file under `rust/` was created, modified or deleted. Every `rust/` access was `sed`,
  `grep`, `awk`, `diff`, `wc`, `ls`.
- No `cargo` command of any kind was run. No build, check, test, clippy, fmt, or
  `sqlx prepare`.
- No `git` mutation. The only `git` invocation was `git -C ... log -1 --format=%h` against the
  snapshot directory (which turned out not to be a repo).
- Files written: this notes file, and edits confined to
  `planning/specs/WP-41-db-quality-pass.md`.
