# WP-82: db.rs module split

**Finding:** ws F42 (severity 1m). Counted inside WP-41's scope; adds 0 to the
finding sum. Escalated from DEFERRED by Michael because `db.rs` "is becoming
problematic due to its size and complexity".

**This is a pure refactor: move code, do not change it.** No SQL edits, no
signature edits, no renames, no bug fixes. Any bug spotted while moving code
gets **reported in the PR description, not fixed** - it almost certainly belongs
to another work package.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose. The symbol inventory in
> `docs/reviews/2026-07-23-rust-review/planning/raw/db-split-inventory.md` was
> measured 2026-07-26 against the live post-WP-41 tree and the tree is still
> under concurrent edit - **re-verify the symbol inventory before starting**;
> functions may have been added or removed since.

## 1. Problem

`rust/web/src/db.rs` is a single module. Measured 2026-07-26 against the live
tree, after WP-41 landed (+1397/-125) - the review's own size numbers are stale:

- 8149 total lines.
- One `#[cfg(all(test, feature = "ssr"))] mod tests`, 4838 lines, **59% of the
  file**, no nested sub-modules, 144 test fns and helpers.
- ~3311 production lines, 107 top-level fns (90 `pub async fn`, 6 `pub fn`, 1
  `pub(crate) fn`, 10 private), 6 `pub struct`, 3 private structs, 2 `pub enum`,
  1 private enum, 2 `pub const`, 1 private const, 2 `impl` blocks.
- 129 item-level `#[cfg(feature = "ssr")]` gates; no module-level gate.
- Largest production fns: `create_game_with_users_tx` (~205), `apply_rating_changes`
  (~151), `update_game_command_success` (~115), `find_game_extended` (~113),
  `choose_colors` (~70), `send_friend_request` (~68).

## 2. Why it's wrong

Be honest about the cost - this is not a spaghetti file:

- The production code is **already de-facto sectioned**, and the module doc
  comment enumerates those sections explicitly under "# Module map". Intra-file
  coupling is low: 28 call edges total across 107 fns, acyclic.
- The **test module is 59% of the file**. Most of the raw line count is tests,
  not tangled production logic.

The justification is therefore narrower than "the file is a mess":

1. **Contention.** Nine remaining web work packages write into this one file
   (see section 6). Every one of them conflicts with every other. Splitting
   first turns those into edits to disjoint files.
2. **Navigability.** A 4838-line test module with no sub-structure means every
   test lookup is a text search, and adding a test means scrolling past fifteen
   unrelated domains.
3. **Review cost.** Diffs against an 8149-line file give reviewers no locality
   signal about which domain changed.

Do not oversell it beyond that. Correctness is not at issue.

## 3. Required end state

### 3a. Layout - decided, do not re-open

`rust/web/src/db/mod.rs` plus `rust/web/src/db/*.rs`, and **`rust/web/src/db.rs`
is deleted**. `mod.rs` style is 7/7 among existing directory modules in
`web/src/` (`auth`, `components`, `email`, `game`, `game_info`, `models`,
`stats`); no `foo.rs` + `foo/` pair exists in the tree.

### 3b. Split axis - by domain

Measured cross-module call edges by axis, over the 28 production edges:

| Axis | Crossing module pairs | Verdict |
|---|---|---|
| By domain | **6** | chosen |
| By operation kind (read / write / tx) | ~19 | worst; every write fn calls a read helper |
| Domain + shared `common` | 6 | this is the chosen axis, with `common` |

All six crossings are acyclic downward dependencies:
`game_write -> {common, users, rating, bots}`, `games -> common`,
`users -> common`, `visibility -> social`.

`rating` stays separate from `game_write` even though merging would drop the
count to 5 pairs: the `elo_*` fns are pure and independently testable.

### 3c. Module table

13 modules (`mod.rs` + 12), plus one test-support module.

| Module | Symbols |
|---|---|
| `db/mod.rs` | module `//!` doc comment, `create_pool`, `pub use crate::game::server_fns::BotSlot;`, the `mod`/`pub use` re-export surface (3d), test-support declaration |
| `db/common.rs` | `build_user_from_row`, `build_game_bot_from_row`, `build_game_type_user`, `build_game_player_from_row`, `normalize_pref_color`, `validate_username`, `cap_digest`, `LocPref`, `remove_highest_prefs`, `choose_colors` |
| `db/game_types.rs` | `find_game_version`, `find_latest_non_deprecated_game_version`, `find_game_type_player_counts`, `find_game_version_rules`, `find_game_version_render_meta`, `find_available_game_types` |
| `db/games.rs` | `find_game`, `GamePlayerExtended` (+ its `impl`), `GameExtended` (+ its `impl`), `find_game_extended`, `is_player_in_game`, `find_active_game_summaries`, `PendingGameRow`, `find_pending_game_summaries`, `FinishedGameRow`, `find_finished_game_summaries`, `find_predecessor_game_id`, `get_all_game_logs`, `get_game_logs`, `find_recent_game_log_lines`, `mark_game_read`, `find_active_turn_games` |
| `db/game_write.rs` | `CreateGameOpts`, `PlayerSlotInternal`, `create_game_with_users`, `create_game_with_users_tx`, `insert_game_logs_tx`, `create_game_logs`, `concede_game`, `concede_game_replace`, `end_game`, `delete_game`, `undo_game`, `StaleStateConflict`, `update_game_command_success` |
| `db/bots.rs` | `BotTurn`, `find_bot_turns`, `find_enabled_bots`, `pick_replacement_bot`, `replacement_bot_available` |
| `db/rating.rs` | `ELO_K`, `elo_transformed_rating`, `elo_expected_score`, `elo_rating_change`, `write_ranked_placings`, `apply_rating_changes` |
| `db/users.rs` | `get_user`, `get_user_by_email`, `get_user_by_name`, `find_user_id_by_name`, `is_user_admin`, `generate_unique_username`, `get_user_name`, `set_user_name`, `get_user_pref_colors`, `set_user_pref_colors`, `get_user_theme`, `set_user_theme`, `get_user_email_prefs`, `set_user_turn_emails_enabled`, `set_user_invite_emails_enabled`, `set_user_reminder_emails_enabled`, `RECENTLY_ACTIVE_WINDOW`, `set_user_last_active`, `active_within_window`, `is_user_recently_active`, `search_users` |
| `db/emails.rs` | `UserEmailRow`, `SWITCH_DIGEST_CAP`, `can_remove_email`, `can_switch_to_email`, `is_expired_unverified`, `list_user_emails`, `find_email_owner`, `insert_unverified_email`, `mark_email_verified`, `SetPrimaryOutcome`, `set_primary_email`, `RemoveEmailOutcome`, `remove_user_email`, `delete_expired_unverified_emails` |
| `db/social.rs` | `FriendRow`, `send_friend_request`, `respond_to_friend_request`, `get_pending_request_source`, `unfriend`, `are_friends_conn`, `should_hide_add_friend`, `list_friends`, `list_incoming_friend_requests`, `count_incoming_friend_requests`, `list_outgoing_friend_requests`, `block_user`, `unblock_user`, `has_block`, `has_block_conn`, `list_blocked`, `opponent_suggestions`, `friends_active_games`, `friends_recent_results` |
| `db/visibility.rs` | `get_invite_policy`, `set_invite_policy`, `get_game_visibility`, `set_game_visibility`, `find_game_visibility_for_users_tx`, `is_game_publicly_visible`, `is_game_visible_to_user`, `check_invite_policy_tx` |
| `db/discovery.rs` | `find_public_index_game_id`, `friend_recent_visible_game`, `recent_games_for_index` |
| `db/proposals.rs` | `find_open_restart_proposal_tx`, `find_open_restart_proposal` |

Cross-cutting placements, decided:

- `find_pending_game_summaries` -> `games.rs` (a games-list UI query, though it
  reads `game_proposals`).
- `find_active_turn_games` -> `games.rs` (a games query, despite only
  reminder-email consumers).
- `opponent_suggestions` -> `social.rs` (`friends` is the dominant join).
- `check_invite_policy_tx` -> `visibility.rs` (it is policy enforcement).
- `cap_digest` -> `common.rs`. It is generic `Vec` truncation with no DB
  involvement; relocating it out of `db` entirely is out of scope here.
- `active_within_window` -> `users.rs`, with presence, not `common.rs`.

Row structs used by exactly one module each move with their code:
`PendingGameRow`, `FinishedGameRow`, `FriendRow`, `UserEmailRow`, `BotTurn`,
`PlayerSlotInternal`. `GameExtended`/`GamePlayerExtended`, `CreateGameOpts` and
`StaleStateConflict` have external consumers but are `pub` and covered by the
re-export, so they also stay with their owning module.

### 3d. Re-export surface - decided, do not re-open

`db/mod.rs` declares each submodule **privately** and glob-re-exports it:

```rust
mod common;
pub use common::*;
// ... one such pair per submodule
```

This keeps every fully-qualified `crate::db::foo(...)` call site working
unchanged. There are 293 `db::` references outside `db.rs` but only **5** `use`
lines that bind a name from `db` anywhere in the tree, and all 5 are covered:
`web/tests/ssr_pages.rs` and `web/tests/nats_bot_eventing.rs`
(`use web::db::{self, CreateGameOpts};`), `web/src/main.rs`
(`use web::db::create_pool;`), `web/src/game/server_fns.rs`
(`use crate::db::CreateGameOpts;`), `web/src/game/mod.rs`
(`use crate::db::{self, CreateGameOpts};`). No caller glob-imports `db`.

Items that were private in `db.rs` but are now called across module boundaries
(`build_*_from_row`, `choose_colors`, `normalize_pref_color`,
`generate_unique_username`'s callers, `apply_rating_changes`,
`write_ranked_placings`, `are_friends_conn`, `has_block_conn`,
`pick_replacement_bot`) become `pub(crate)` at minimum. Do not widen anything to
`pub` that was not `pub` before.

### 3e. Gating - decided, do not re-open

Keep the existing **per-item** `#[cfg(feature = "ssr")]` gates verbatim. **Do
not introduce module-level `ssr` gates.** `validate_username` is deliberately
ungated so the client-side settings form and the server fns share one
definition; a module-level gate on `common.rs` would break the client build.

Item-level attributes travel with their functions unchanged: the 3
`#[allow(clippy::too_many_arguments)]` (`build_game_type_user`,
`build_game_player_from_row`, `update_game_command_success`) and the 12
`#[tracing::instrument(...)]`.

`Result` is `anyhow::Result` (`use anyhow::Result;` in the current file header,
Lead-verified). There is no crate-local alias to move; each submodule imports
what it needs (`anyhow::Result`, `sqlx::postgres::PgPool`, `uuid::Uuid`,
`crate::models::user::User`, `crate::models::game::Game`,
`crate::game::StatusUpdate`, `time::PrimitiveDateTime`), each with the same
`#[cfg(feature = "ssr")]` gating the current imports carry.

### 3f. The module doc comment

The `//!` block moves to `db/mod.rs` and is **partly rewritten**:

- **Preserve verbatim** the `updated_at` trigger convention section (which
  tables have the BEFORE UPDATE trigger, which need manual `updated_at`, the
  three conditional triggers). It applies tree-wide and must not be lost.
- **Rewrite** the "# Module map" section. Its current text describes one file
  and its section order, and names ws F42 as deferred. Replace it with a map of
  the new modules from the table in 3c.
- **Preserve** the `ssr`-gating note including the explicit `validate_username`
  carve-out and the note that the other pure predicates stay gated. Echo the
  carve-out in a short `//!` header on `db/common.rs`.

### 3g. Tests

Each module gets its own `#[cfg(all(test, feature = "ssr"))] mod tests`
containing the tests that bind to it. The name-prefixed clusters make the
binding mechanical: `choose_colors_*` -> `common.rs`; `elo_*` and the
rating/finishing tests -> `rating.rs`; `find_public_index_game_id_*` ->
`discovery.rs`; `friend_*` / `block_*` / `unfriend_*` -> `social.rs`;
`is_game_visible_*` / `game_visibility_*` -> `visibility.rs`;
`update_game_command_success_*` and `delete_game_*` -> `game_write.rs`;
`search_users_*` -> `users.rs`; the email tests (`set_primary_*`, `can_*`,
`cap_digest_*` - `cap_digest_*` goes with `common.rs` where the fn lives) ->
`emails.rs`; `*_summaries_*` -> `games.rs`.

Only three private production items are directly exercised by tests -
`choose_colors`, `elo_rating_change`, `apply_rating_changes` - so those tests
must land in `common.rs` and `rating.rs` respectively. Everything else binds to
`pub` API and can go wherever it reads best.

The ~12 shared fixture helpers (`make_user`, `make_game_type_and_version`,
`make_game_with_players`, `make_proposal`, `add_proposal_player`,
`insert_proposal`, `finish_game`, `set_recently_active`, `set_stale`,
`count_rows`, `position_of`, `check_roster`) hoist into one shared module,
**`rust/web/src/db/test_support.rs`**, declared in `db/mod.rs` as:

```rust
#[cfg(all(test, feature = "ssr"))]
mod test_support;
```

Its helpers are `pub(crate)` so each module's `mod tests` can `use` them. It is
**not** re-exported from `mod.rs`. This is the main coupling risk in the test
split - do the helper hoist first, then move test clusters one module at a time.

### 3h. `friend_recent_visible_game`

It deliberately inlines the `is_game_visible_to_user` predicate as SQL rather
than calling it, with a doc comment cross-reference and a drift-guard test
asserting the two agree. After the split the two live in different modules
(`db/discovery.rs` and `db/visibility.rs`). **Update the cross-reference doc
comment to name the new path.** Keep the drift-guard test; place it with
whichever of the two modules reads better and have it reference the other by
full path. **Do not de-duplicate the inlined SQL** - that is a separate concern.

## 4. Non-goals

- No behaviour change of any kind.
- No query rewriting. SQL text must be byte-identical after the move.
- No signature changes, no renames, no visibility widening beyond what 3d
  requires.
- No bug fixing during the move. Report, do not fix.
- No inventing a transaction abstraction. Every write opens `pool.begin()`
  inline today; that stays.
- No collapsing the `*_conn` / `*_tx` duplicate pairs (`has_block` /
  `has_block_conn`, `find_open_restart_proposal` /
  `find_open_restart_proposal_tx`, `create_game_with_users` /
  `create_game_with_users_tx`, `create_game_logs` / `insert_game_logs_tx`).
- No decomposing the large functions (`create_game_with_users_tx`,
  `apply_rating_changes`, `update_game_command_success`).
- No moving `validate_username`, `normalize_pref_color` or `cap_digest` out of
  `db`, and no `sqlx::FromRow` conversion of the positional row mappers.
- No widening into the deferred architectural review (section H of the
  inventory) - repository boundary, `BotSlot`/`StatusUpdate` dependency
  inversion, `is_user_recently_active` swallowing errors, moving tests to
  `web/tests/`. All of those stay filed as observations.

## 5. Regression test cases

A pure move is verifiable mechanically. The implementer must show all of:

- **Same test names, all still present and passing.** Collect the sorted set of
  test fn names before and after; the two sets must be identical. Nothing
  renamed, nothing dropped, nothing added.
- **Diff shape.** `git diff --stat` shows `db.rs` deleted and the new `db/`
  files added, with a **near-zero net line delta** outside module headers. The
  only legitimate additions are: per-file `use` blocks, the `mod` / `pub use`
  pairs in `db/mod.rs`, per-module `mod tests` wrappers, the rewritten "Module
  map", and `pub(crate)` visibility bumps. If net lines grew materially,
  something was rewritten rather than moved.
- **No `cargo sqlx prepare` needed.** `rust/web/.sqlx/` holds 132 entries keyed
  `query-<sha256-of-query-text>.json`, and an entry's only top-level fields are
  `db_name`, `describe`, `hash`, `query`. **No path, line number or module is
  recorded anywhere in the cache**, so moving `sqlx::query!` invocations between
  files cannot invalidate it. Re-prepare is required only if SQL *text* changes
  - and per section 4 it must not. All new modules stay in the `web` crate, so
  the per-crate `.sqlx` location is unaffected. If the build demands a
  re-prepare, that is evidence a query was edited: stop and find it.
- **Public `db::` surface unchanged.** All 293 external `db::` references and
  all 5 `use` lines compile without edit. Zero changes to files outside
  `rust/web/src/db/` in this PR, other than the deletion of `db.rs`.
- **Client build still compiles.** The non-`ssr` build must still see
  `validate_username`; a module-level gate regression shows up here.

## 6. Ordering

**This lands BEFORE the db.rs-touching packages, as a hard predecessor.** This
is **inverted** from the old note on WP-78, which had the split land last. The
inversion is deliberate: those packages rebase onto the new layout, instead of
the split rebasing onto ten sets of moved-file edits.

`WP-78 db.rs module split - DEFERRED` in the "Unowned" section of
`work-packages.md` is the same item and is **SUPERSEDED BY WP-82**. Do not work
from that stale entry. The `WP-50 -> WP-78` edge in `landing-order.md` 6.4 is
likewise inverted - WP-82 precedes WP-50.

Hard predecessor for (all list `rust/web/src/db.rs` in their paths): **WP-35,
WP-40, WP-45, WP-47, WP-49, WP-50, WP-52, WP-53, WP-59**. Also for **WP-42**,
which consumes WP-47's db.rs predicate.

**WP-41** (db.rs quality pass) has already landed; nothing blocks WP-82.
