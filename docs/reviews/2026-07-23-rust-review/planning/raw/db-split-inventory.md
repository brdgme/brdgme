# db.rs split inventory (evidence base)

Target: `/home/beefsack/Development/brdgme/rust/web/src/db.rs`
Read-only inventory. Code identified by symbol name only; line numbers appear
only as sizes/counts in section A.

## A. Measurements

| Metric | Value | Command |
|---|---|---|
| Total lines | 8149 | `wc -l db.rs` |
| Test module | single `#[cfg(all(test, feature = "ssr"))] mod tests` | `grep -n 'mod tests' db.rs` -> one hit |
| Test module size | 4838 lines (~59% of file) | 8149 minus the production span |
| Production (non-test) lines | ~3311 | remainder |
| `pub async fn` (top level) | 90 | `grep -c '^pub async fn ' db.rs` |
| `pub fn` (top level) | 6 | `grep -c '^pub fn ' db.rs` |
| `pub(crate) fn` (top level) | 1 (`normalize_pref_color`) | `grep -n '^pub(crate)' db.rs` |
| private top-level `fn` | 10 | derived from the item listing below |
| Total top-level fns (production) | 107 | `grep -oP '^(pub )?(async )?fn \K\w+'` over the production span |
| All `fn` incl. test fns/helpers | 255 | `grep -c '^\s*\(pub \)\?\(async \)\?fn '` |
| Test fns + test helpers | 144 | `grep -c '    async fn \|    fn ' db.rs` |
| `pub struct` | 6 | `grep -c '^pub struct '` |
| private `struct` | 3 (`PendingGameRow`, `FinishedGameRow`, `FriendRow`) | item listing |
| `pub enum` | 2 (`SetPrimaryOutcome`, `RemoveEmailOutcome`) | `grep -c '^pub enum '` |
| private `enum` | 1 (`PlayerSlotInternal`) | item listing |
| `type` alias | 1 private (`LocPref`) | `grep -c '^type '` |
| `pub const` | 2 (`RECENTLY_ACTIVE_WINDOW`, `SWITCH_DIGEST_CAP`) | item listing |
| private `const` | 1 (`ELO_K`) | item listing |
| `impl` blocks | 2 (`GamePlayerExtended`, `GameExtended`) | item listing |
| `pub use` re-export | 1 (`pub use crate::game::server_fns::BotSlot;`) | item listing |
| `#[cfg(feature = "ssr")]` item gates | 129 | `awk '/^#\[/' \| sort \| uniq -c` |

Sizes measured with:
`awk 'NR<3311 && /^(pub )?(async )?fn |^(pub )?(struct|enum|impl|const|type) /{...}' db.rs | sort -rn`
(distance to next top-level item; approximate, includes preceding doc comments
of the following item excluded).

### 10 largest production functions

| Rank | Function | ~lines |
|---|---|---|
| 1 | `create_game_with_users_tx` | 205 |
| 2 | `apply_rating_changes` | 151 |
| 3 | `update_game_command_success` | 115 |
| 4 | `find_game_extended` | 113 |
| 5 | `choose_colors` | 70 |
| 6 | `send_friend_request` | 68 |
| 7 | `find_active_game_summaries` | 66 |
| 8 | `undo_game` | 61 |
| 9 | `build_game_type_user` | 56 |
| 10 | `find_finished_game_summaries` | 55 |
| (tie) | `concede_game` | 55 |

### Test modules

Exactly **one** `mod tests`, gated `#[cfg(all(test, feature = "ssr"))]`, no
nested sub-modules (`awk 'NR>3311' db.rs | grep '^    mod '` -> no hits).
It covers every domain in the file: migrations/pool, presence, friends/blocks,
invite policy, game visibility, index/discovery queries, suggestions, game
summaries, game lifecycle, command write path, ELO, colours, deletion, user
search, user settings, and multi-email.

## B. Full symbol inventory (source order)

The file **already has de-facto sections** and its module doc comment
enumerates them explicitly ("# Module map"). The existing ordering is largely a
domain grouping already, with one exception: the row builders sit at the top as
a shared prelude, and the getters/lookups section mixes user, game-version and
game-summary reads.

Legend: kind `A` = `pub async fn`, `a` = private `async fn`, `F` = `pub fn`,
`f` = private `fn`, `Fc` = `pub(crate) fn`.

### S1 - Row builders (declared private, shared prelude)

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `BotSlot` | `pub use` re-export from `crate::game::server_fns` | bot slot type for game creation | - |
| `build_user_from_row` | f | maps a query row into `User` | (row mapper) |
| `build_game_bot_from_row` | f | maps a query row into game bot | (row mapper) |
| `build_game_type_user` | f | builds game-type-user rating record, defaults 1200 | (row mapper) |
| `build_game_player_from_row` | f | maps a query row into game player | (row mapper) |

### S2 - Pool + lookups/getters

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `create_pool` | A | builds the `PgPool` from env config | - |
| `get_user_by_email` | A | user lookup by any of their emails | users, user_emails |
| `get_user` | A | user lookup by id | users |
| `find_game_version` | A | one game version by id | game_versions |
| `find_latest_non_deprecated_game_version` | A | newest live version for a type | game_versions |
| `find_game_type_player_counts` | A | min/max player counts per type | game_types, game_versions |
| `find_game_version_rules` | A | rules markdown for a version | game_versions |
| `find_game_version_render_meta` | A | render metadata for a version | game_versions |
| `find_available_game_types` | A | list playable types with weight/blurb | game_types, game_versions |
| `find_game` | A | one game row by id | games |
| `GamePlayerExtended` | pub struct | player + user + bot + rating bundle | - |
| `impl GamePlayerExtended` | impl | accessors on the bundle | - |
| `GameExtended` | pub struct | game + version + players bundle | - |
| `impl GameExtended` | impl | accessors on the bundle | - |
| `find_game_extended` | A | full game aggregate load (hot path) | game_types, game_players, users, game_type_users, game_bots |
| `BotTurn` | pub struct | bot whose turn it is | - |
| `find_bot_turns` | A | bots currently on turn in a game | game_players, game_bots |
| `find_enabled_bots` | A | enabled bot names | bots |
| `is_player_in_game` | A | membership predicate | game_players |
| `is_user_admin` | A | admin flag lookup | users |
| `find_user_id_by_name` | A | user id by username | users |
| `find_active_game_summaries` | A | active-games list for a user | games, game_versions, game_types, game_players, users, game_bots |
| `PendingGameRow` | struct (priv) | row shape for pending summaries | - |
| `find_pending_game_summaries` | A | open proposals list for a user | game_proposals, game_versions, game_types, game_proposal_players, users |
| `FinishedGameRow` | struct (priv) | row shape for finished summaries | - |
| `find_finished_game_summaries` | A | recent finished games for a user | games, game_versions, game_types, game_players, users, game_bots |
| `find_predecessor_game_id` | A | game that restarted into this one | games |
| `find_open_restart_proposal_tx` | A | earliest open restart proposal (tx) | game_proposals |
| `find_open_restart_proposal` | A | pool wrapper of the above | game_proposals |

### S3 - Username, colours, creation options

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `CreateGameOpts<'a>` | pub struct | parameter bundle for game creation | - |
| `PlayerSlotInternal` | enum (priv) | human vs bot slot during creation | - |
| `validate_username` | F (**ungated**, shared client/server) | username charset/length predicate | - |
| `generate_unique_username` | A | petname loop until unclaimed | users |
| `normalize_pref_color` | Fc | canonicalises legacy colour names | - |
| `LocPref` | type (priv) | `(usize, Vec<String>)` colour pref pair | - |
| `remove_highest_prefs` | f | one step of colour preference resolution | - |
| `choose_colors` | f | assigns distinct colours from prefs+palette | - |

### S4 - Game lifecycle writes

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `create_game_with_users` | A | pool wrapper around the tx version | - |
| `create_game_with_users_tx` | A | creates game, players, bots, colours (largest fn) | users, user_emails, games, game_versions, game_players, game_type_users, game_bots |
| `insert_game_logs_tx` | A | writes logs and their targets | game_players, game_logs, game_log_targets |
| `create_game_logs` | A | pool wrapper for log insertion | - |
| `concede_game` | A | marks conceder out, may finish + rate | games, game_players, game_logs |
| `pick_replacement_bot` | A | selects a bot allowed to replace humans | bots, game_bots |
| `replacement_bot_available` | A | any replacement-capable bot enabled | bots |
| `concede_game_replace` | A | swaps a conceding human for a bot | game_players, game_logs |
| `end_game` | A | force-finish, rank and rate | games, game_players, game_logs |
| `delete_game` | A | cascade delete + null proposal FKs | games, game_proposals, game_log_targets, game_logs, game_players, game_bots |
| `mark_game_read` | A | clears unread flag for one player | game_players |
| `undo_game` | A | restores stashed state, clears undo | games, game_players, game_logs |
| `get_all_game_logs` | A | all logs for a game | game_logs |
| `get_game_logs` | A | logs visible to a viewer | game_logs, game_log_targets |

### S5 - ELO / ranking

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `ELO_K` | const (priv) | K-factor 32.0 | - |
| `elo_transformed_rating` | f | 10^(r/400) transform | - |
| `elo_expected_score` | f | pairwise expected score | - |
| `elo_rating_change` | f | pairwise integer rating delta | - |
| `write_ranked_placings` | a | writes dense ranked placings | game_players |
| `apply_rating_changes` | a | pairwise rating update, skips pure-bot games | game_players, games, game_versions, game_type_users |

### S6 - Command write path

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `StaleStateConflict` | pub struct (`thiserror`) | optimistic-concurrency error | - |
| `update_game_command_success` | A | persists a successful command: state, turns, logs, undo stash, finish+rate | games, game_players (+ via callees) |

### S7 - Theme and presence

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `get_user_theme` | A | read theme preference | users |
| `set_user_theme` | A | write theme preference | users |
| `RECENTLY_ACTIVE_WINDOW` | pub const | 600s presence window | - |
| `set_user_last_active` | A | stamp `last_active_at` | users |
| `active_within_window` | F | pure presence predicate | - |
| `is_user_recently_active` | A | presence check (swallows errors) | users |

### S8 - Friends and blocks (#30)

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `FriendRow` | struct (priv) | friends row shape | - |
| `send_friend_request` | A | create/auto-accept/no-op request | blocks, friends |
| `respond_to_friend_request` | A | accept or decline a pending row | friends |
| `get_pending_request_source` | A | who sent the pending request | friends |
| `unfriend` | A | delete accepted friendship | friends |
| `are_friends_conn` | A | friendship predicate on a connection | friends |
| `should_hide_add_friend` | A | UI predicate over all row states | friends |
| `list_friends` | A | accepted friends with names | friends, users |
| `list_incoming_friend_requests` | A | pending inbound | friends, users |
| `count_incoming_friend_requests` | A | pending inbound count | friends |
| `list_outgoing_friend_requests` | A | pending outbound | friends, users |
| `block_user` | A | block and sever friendship | blocks, friends |
| `unblock_user` | A | remove block | blocks |
| `has_block` | A | block predicate (pool) | blocks |
| `has_block_conn` | A | block predicate (connection) | blocks |
| `list_blocked` | A | blocked users with names | blocks, users |

### S9 - Invite policy and game visibility

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `get_invite_policy` | A | read invite policy | users |
| `set_invite_policy` | A | write invite policy | users |
| `get_game_visibility` | A | read game-visibility setting | users |
| `set_game_visibility` | A | write game-visibility setting | users |
| `find_game_visibility_for_users_tx` | A | bulk visibility lookup (tx) | users |
| `is_game_publicly_visible` | A | all human players public | game_players, users |
| `is_game_visible_to_user` | A | per-viewer visibility predicate | game_players, users, friends |
| `find_public_index_game_id` | A | pick a public game for the index | games, game_players, users |
| `find_recent_game_log_lines` | A | last N log lines for a game | game_logs |
| `friend_recent_visible_game` | A | most recent friend game viewer may see (visibility predicate inlined) | game_players, users, friends, games, game_versions, game_types |
| `recent_games_for_index` | A | last 10 games for index page | games, game_versions, game_types, game_players |
| `check_invite_policy_tx` | A | enforces invite policy + blocks at creation | user_emails, users |

### S10 - User search and suggestions

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `get_user_by_name` | A | id+name by username | users |
| `search_users` | A | prefix search, escapes LIKE, excludes blocked/self | users, blocks |
| `opponent_suggestions` | A | friends first then recent co-players | friends, users, games, game_players, blocks |
| `friends_active_games` | A | friends' in-progress games | games, game_versions, game_types, game_players, users, friends |
| `friends_recent_results` | A | friends' recent finished results | games, game_versions, game_types, game_players, users, friends |

### S11 - User settings

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `get_user_name` / `set_user_name` | A / A | username read/write | users |
| `get_user_pref_colors` / `set_user_pref_colors` | A / A | colour prefs read/write | users |
| `get_user_email_prefs` | A | three email toggles | users |
| `set_user_turn_emails_enabled` | A | toggle turn emails | users |
| `set_user_invite_emails_enabled` | A | toggle invite emails | users |
| `set_user_reminder_emails_enabled` | A | toggle reminder emails | users |

### S12 - Multiple emails per account (#22d)

| Symbol | Kind | Description | Tables |
|---|---|---|---|
| `UserEmailRow` | pub struct | one user_emails row | - |
| `SWITCH_DIGEST_CAP` | pub const | digest cap = 20 | - |
| `can_remove_email` | F | pure: only non-primary removable | - |
| `can_switch_to_email` | F | pure: requires verified | - |
| `is_expired_unverified` | F | pure: expiry predicate | - |
| `cap_digest<T>` | F | pure: truncate a Vec to cap | - |
| `list_user_emails` | A | all emails for a user | user_emails |
| `find_email_owner` | A | owner of an email address | user_emails |
| `insert_unverified_email` | A | add pending email, rejects taken | user_emails |
| `mark_email_verified` | A | stamp verified_at | user_emails |
| `SetPrimaryOutcome` | pub enum | result of primary switch | - |
| `set_primary_email` | A | move primary flag, exactly one | user_emails |
| `RemoveEmailOutcome` | pub enum | result of removal | - |
| `remove_user_email` | A | remove a non-primary email | user_emails |
| `find_active_turn_games` | A | games awaiting a user (reminder emails) | game_players, games |
| `delete_expired_unverified_emails` | A | sweep expired pending emails | user_emails |

### S13 - Tests

`mod tests` (single, `#[cfg(all(test, feature = "ssr"))]`), 144 fns.

## C. Intra-file coupling

### Call edges (caller -> callee), production code only

| Caller | Callee | Cross-section? |
|---|---|---|
| `find_game_extended` | `build_user_from_row` | S2 -> S1 |
| `find_game_extended` | `build_game_player_from_row` | S2 -> S1 |
| `find_game_extended` | `build_game_bot_from_row` | S2 -> S1 |
| `find_game_extended` | `build_game_type_user` | S2 -> S1 |
| `find_game_extended` | `find_game` | S2 -> S2 |
| `find_game_extended` | `find_game_version` | S2 -> S2 |
| `generate_unique_username` | `validate_username` | S3 -> S3 |
| `choose_colors` | `remove_highest_prefs` | S3 -> S3 |
| `choose_colors` | `normalize_pref_color` | S3 -> S3 |
| `get_user_pref_colors` | `normalize_pref_color` | S11 -> S3 |
| `create_game_with_users` | `create_game_with_users_tx` | S4 -> S4 |
| `create_game_with_users_tx` | `choose_colors` | S4 -> S3 |
| `create_game_with_users_tx` | `generate_unique_username` | S4 -> S3 |
| `create_game_logs` | `insert_game_logs_tx` | S4 -> S4 |
| `concede_game` | `apply_rating_changes` | S4 -> S5 |
| `concede_game_replace` | `pick_replacement_bot` | S4 -> S4 |
| `end_game` | `write_ranked_placings` | S4 -> S5 |
| `end_game` | `apply_rating_changes` | S4 -> S5 |
| `elo_expected_score` | `elo_transformed_rating` | S5 -> S5 |
| `elo_rating_change` | `elo_expected_score` | S5 -> S5 |
| `apply_rating_changes` | `elo_rating_change` | S5 -> S5 |
| `update_game_command_success` | `insert_game_logs_tx` | S6 -> S4 |
| `update_game_command_success` | `write_ranked_placings` | S6 -> S5 |
| `update_game_command_success` | `apply_rating_changes` | S6 -> S5 |
| `is_user_recently_active` | `active_within_window` | S7 -> S7 |
| `check_invite_policy_tx` | `are_friends_conn` | S9 -> S8 |
| `check_invite_policy_tx` | `has_block_conn` | S9 -> S8 |
| `remove_user_email` | `can_remove_email` | S12 -> S12 |

Total: 28 edges. 26 distinct caller/callee pairs plus the two `find_game_extended`
row-builder duplicates counted once each.

Note: `friend_recent_visible_game` deliberately **inlines** the
`is_game_visible_to_user` predicate rather than calling it (documented in its
doc comment; a test asserts the two agree). This is duplicated logic, not an
edge.

### Shared helpers/types used across many functions

| Symbol | Nature | Consumers |
|---|---|---|
| `sqlx::PgPool` | plumbing | ~85 of 107 fns take `pool: &PgPool` |
| `sqlx::PgConnection` / `&mut *tx` | plumbing | `*_tx` fns, `are_friends_conn`, `has_block_conn`, `generate_unique_username`, `write_ranked_placings`, `apply_rating_changes` |
| `Result<T>` (crate-level alias, `anyhow`/`eyre`-style - **unverified which**) | error type | every fallible fn |
| `uuid::Uuid` | plumbing | nearly all fns |
| `crate::models::user::User` | row type | `build_user_from_row`, `get_user*`, `find_game_extended` |
| `crate::models::game::Game` | row type | `find_game`, `find_game_extended` |
| `crate::game::StatusUpdate` | domain type | command write path |
| `build_*_from_row` (4 fns) | row mappers | currently only `find_game_extended` |
| `GameExtended` / `GamePlayerExtended` | aggregate types | S2, plus external `email/notify.rs`, `game/server_fns.rs` |
| `StaleStateConflict` | error type | S6 only, plus external `game/mod.rs` |
| `normalize_pref_color` | pure helper | S3, S11, plus external `stats/queries.rs`, `email/commands.rs` |
| `CreateGameOpts` / `BotSlot` / `PlayerSlotInternal` | param types | S4, plus 5 external files |

There is **no** sqlx type alias and **no** generic transaction helper. Every
transaction is opened ad hoc via `pool.begin()` inside the individual write fn.

## D. External callers

Total `db::` references outside `db.rs`: **293**
(`rg -o 'db::' --glob '!web/src/db.rs' -t rust | wc -l`)

### Call sites per file

| File | `db::` refs | Domains touched |
|---|---|---|
| `/home/beefsack/Development/brdgme/rust/web/src/game/server_fns.rs` | 80 | games (lookups, summaries, lifecycle, proposals, visibility) |
| `/home/beefsack/Development/brdgme/rust/web/src/email/commands.rs` | 48 | games, users/settings, emails, bots |
| `/home/beefsack/Development/brdgme/rust/web/src/auth/server.rs` | 28 | users/settings, emails |
| `/home/beefsack/Development/brdgme/rust/web/src/game/mod.rs` | 25 | games, command write path |
| `/home/beefsack/Development/brdgme/rust/web/src/friends.rs` | 25 | friends/blocks, invite policy, visibility, search |
| `/home/beefsack/Development/brdgme/rust/web/src/email/notify.rs` | 14 | games (extended + logs) |
| `/home/beefsack/Development/brdgme/rust/web/src/proposals.rs` | 11 | game types/versions, invite policy, usernames |
| `/home/beefsack/Development/brdgme/rust/web/src/email/inbound.rs` | 10 | games, users |
| `/home/beefsack/Development/brdgme/rust/web/tests/nats_bot_eventing.rs` | 9 | games |
| `/home/beefsack/Development/brdgme/rust/web/tests/ssr_pages.rs` | 7 | games |
| `/home/beefsack/Development/brdgme/rust/web/src/rules.rs` | 7 | game versions |
| `/home/beefsack/Development/brdgme/rust/web/src/game/import.rs` | 7 | games, usernames |
| `/home/beefsack/Development/brdgme/rust/web/src/email/outbound.rs` | 4 | users (theme, presence) |
| `/home/beefsack/Development/brdgme/rust/web/src/index.rs` | 3 | discovery/index, friends |
| `/home/beefsack/Development/brdgme/rust/web/src/game/export.rs` | 2 | games, admin |
| `/home/beefsack/Development/brdgme/rust/web/src/email/sweep.rs` | 2 | emails, games |
| `/home/beefsack/Development/brdgme/rust/web/src/admin.rs` | 2 | users (admin) |
| `/home/beefsack/Development/brdgme/rust/web/src/stats/queries.rs` | 1 | colours (`normalize_pref_color`) |
| `/home/beefsack/Development/brdgme/rust/web/src/stats/mod.rs` | 1 | friends (`should_hide_add_friend`) |
| `/home/beefsack/Development/brdgme/rust/web/src/main.rs` | 1 | pool |
| `/home/beefsack/Development/brdgme/rust/web/src/bin/import_game.rs` | 1 | pool |
| `/home/beefsack/Development/brdgme/rust/web/src/app.rs` | 1 | presence const |

### Import style - decisive for re-exports

`rg -n 'use .*\bdb\b' --glob '!web/src/db.rs' -t rust` returns only **5** import
lines in the whole tree:

- `web/tests/ssr_pages.rs`: `use web::db::{self, CreateGameOpts};`
- `web/tests/nats_bot_eventing.rs`: `use web::db::{self, CreateGameOpts};`
- `web/src/main.rs`: `use web::db::create_pool;`
- `web/src/game/server_fns.rs`: `use crate::db::CreateGameOpts;`
- `web/src/game/mod.rs`: `use crate::db::{self, CreateGameOpts};`

Everything else is **fully-qualified `crate::db::foo(...)` at the call site**.

**Conclusion:** `pub use` re-exports from a `db/mod.rs` keep 100% of external
callers compiling unchanged. Only 4 type imports (`CreateGameOpts`, plus
`create_pool` by name) are name-bound, and those are covered by the same
re-export. No caller does a glob import.

### Most-used symbols externally

`find_game_extended` (43), `create_game_with_users` (25), `CreateGameOpts` (15),
`find_game_type_player_counts` (9), `GameExtended` (7), `find_game_version` (7),
`SetPrimaryOutcome` (6), `RemoveEmailOutcome` (6), `is_user_admin` (6),
`get_user_theme` (6). Everything else is <= 5.

## E. Split-axis recommendation

### Axis comparison

| Axis | Cross-module edges (of 28) | Notes |
|---|---|---|
| (i) By domain | **6** | see mapping below |
| (ii) By operation kind (read / write / tx) | **~19** | every write fn calls a read helper or an internal read; `find_game_extended`->row builders, `create_game_with_users_tx`->`generate_unique_username`, `check_invite_policy_tx`->predicates, all lifecycle->ELO edges cross. Worst axis. |
| (iii) Hybrid (domain + a shared `common`) | **6** (same as (i), with row builders/pure helpers pulled into `common`) | marginal gain over (i); adds a module |

**Recommendation: (i) by domain**, with the four row builders and the pure
helpers kept in a small `db/common.rs` (this is the hybrid's `common`, so
effectively (i)+(common) = (iii) minus the operation-kind axis).

### Proposed module list

| Module | Contents (from section B) | Notes |
|---|---|---|
| `db/mod.rs` | `create_pool`, all `pub use` re-exports, module doc comment | the compatibility surface |
| `db/common.rs` | `build_user_from_row`, `build_game_bot_from_row`, `build_game_type_user`, `build_game_player_from_row`, `normalize_pref_color`, `validate_username`, `cap_digest`, `LocPref`, `remove_highest_prefs`, `choose_colors` | pure + row mapping; `validate_username` must stay **ungated** |
| `db/game_types.rs` | `find_game_version`, `find_latest_non_deprecated_game_version`, `find_game_type_player_counts`, `find_game_version_rules`, `find_game_version_render_meta`, `find_available_game_types` | reads `game_types` / `game_versions` only; zero internal edges |
| `db/games.rs` | `find_game`, `GamePlayerExtended`(+impl), `GameExtended`(+impl), `find_game_extended`, `is_player_in_game`, `find_active_game_summaries`, `PendingGameRow`, `find_pending_game_summaries`, `FinishedGameRow`, `find_finished_game_summaries`, `find_predecessor_game_id`, `get_all_game_logs`, `get_game_logs`, `find_recent_game_log_lines`, `mark_game_read`, `find_active_turn_games` | game reads |
| `db/game_write.rs` | `CreateGameOpts`, `PlayerSlotInternal`, `create_game_with_users`, `create_game_with_users_tx`, `insert_game_logs_tx`, `create_game_logs`, `concede_game`, `concede_game_replace`, `end_game`, `delete_game`, `undo_game`, `StaleStateConflict`, `update_game_command_success` | the whole write path; keeps S4 and S6 together, which kills 3 edges |
| `db/bots.rs` | `find_bot_turns`, `find_enabled_bots`, `pick_replacement_bot`, `replacement_bot_available`, `BotTurn` | `bots` / `game_bots` tables |
| `db/rating.rs` | `ELO_K`, `elo_transformed_rating`, `elo_expected_score`, `elo_rating_change`, `write_ranked_placings`, `apply_rating_changes` | 3 internal edges stay internal |
| `db/users.rs` | `get_user`, `get_user_by_email`, `get_user_by_name`, `find_user_id_by_name`, `is_user_admin`, `generate_unique_username`, `get_user_name`, `set_user_name`, `get_user_pref_colors`, `set_user_pref_colors`, `get_user_theme`, `set_user_theme`, `get_user_email_prefs`, `set_user_turn_emails_enabled`, `set_user_invite_emails_enabled`, `set_user_reminder_emails_enabled`, `RECENTLY_ACTIVE_WINDOW`, `set_user_last_active`, `active_within_window`, `is_user_recently_active`, `search_users` | all `users`-table CRUD + presence |
| `db/emails.rs` | `UserEmailRow`, `SWITCH_DIGEST_CAP`, `can_remove_email`, `can_switch_to_email`, `is_expired_unverified`, `list_user_emails`, `find_email_owner`, `insert_unverified_email`, `mark_email_verified`, `SetPrimaryOutcome`, `set_primary_email`, `RemoveEmailOutcome`, `remove_user_email`, `delete_expired_unverified_emails` | `user_emails` table only |
| `db/social.rs` | `FriendRow`, `send_friend_request`, `respond_to_friend_request`, `get_pending_request_source`, `unfriend`, `are_friends_conn`, `should_hide_add_friend`, `list_friends`, `list_incoming_friend_requests`, `count_incoming_friend_requests`, `list_outgoing_friend_requests`, `block_user`, `unblock_user`, `has_block`, `has_block_conn`, `list_blocked`, `opponent_suggestions`, `friends_active_games`, `friends_recent_results` | `friends` + `blocks`; suggestions/feeds live here because they all join `friends` |
| `db/visibility.rs` | `get_invite_policy`, `set_invite_policy`, `get_game_visibility`, `set_game_visibility`, `find_game_visibility_for_users_tx`, `is_game_publicly_visible`, `is_game_visible_to_user`, `check_invite_policy_tx` | policy predicates |
| `db/discovery.rs` | `find_public_index_game_id`, `friend_recent_visible_game`, `recent_games_for_index` | index-page feeds |
| `db/proposals.rs` | `find_open_restart_proposal_tx`, `find_open_restart_proposal` | `game_proposals` reads (note `find_pending_game_summaries` also reads them but is a games-list query - see cross-cutting) |

**13 modules** (12 + `mod.rs`).

### Cross-module edges under the recommended axis (6)

| Caller (module) | Callee (module) |
|---|---|
| `find_game_extended` (games) | `build_*_from_row` x4 (common) - counts as 4 edges |
| `get_user_pref_colors` (users) | `normalize_pref_color` (common) |
| `create_game_with_users_tx` (game_write) | `choose_colors` (common) |
| `create_game_with_users_tx` (game_write) | `generate_unique_username` (users) |
| `concede_game` / `end_game` / `update_game_command_success` (game_write) | `apply_rating_changes`, `write_ranked_placings` (rating) - 5 edges |
| `check_invite_policy_tx` (visibility) | `are_friends_conn`, `has_block_conn` (social) - 2 edges |
| `concede_game_replace` (game_write) | `pick_replacement_bot` (bots) |

Counting each concrete edge: 4 (row builders) + 1 + 1 + 1 + 5 + 2 + 1 = **15**
concrete edges, but only **6 distinct module-pair dependencies**, and all six
are acyclic downward dependencies (`game_write -> {common, users, rating, bots}`,
`games -> common`, `users -> common`, `visibility -> social`). No cycles.

Merging `rating` into `game_write` would drop it to 5 module pairs; keeping it
separate is preferable because ELO is independently testable and the pure `elo_*`
fns have no DB dependency at all.

### Genuinely cross-cutting functions

| Function | Why cross-cutting | Recommended home |
|---|---|---|
| `find_pending_game_summaries` | joins `game_proposals` + `game_versions` + users, but is a games-list UI query | `db/games.rs` (not `proposals`) |
| `friend_recent_visible_game` | joins friends + visibility + games | `db/discovery.rs`; it inlines the visibility predicate |
| `opponent_suggestions` | friends + games + blocks | `db/social.rs` (friends is the dominant join) |
| `find_active_turn_games` | games query, but its only consumers are the reminder-email paths | `db/games.rs` |
| `check_invite_policy_tx` | reads `user_emails`+`users`, calls friends/blocks | `db/visibility.rs` (it is policy enforcement) |
| `normalize_pref_color` | used by colour assignment, user settings, stats, email | `db/common.rs` |
| `validate_username` | the only ungated item; shared client/server | `db/common.rs`, must keep no `ssr` gate |
| `cap_digest` | generic Vec truncation, not DB at all | `db/common.rs` (or arguably out of `db` entirely) |

## F. What must stay central (`db/mod.rs` + `db/common.rs`)

Concrete list of the shared layer:

**Must be in `db/mod.rs`:**
- the module-level `//!` doc comment (see G) - the `updated_at` trigger
  convention applies file-wide and must not be lost
- `create_pool`
- `pub use crate::game::server_fns::BotSlot;` (existing re-export)
- `pub use` re-exports of **every** currently-`pub` symbol, so
  `crate::db::foo(...)` call sites keep working (see D)

**Must be in `db/common.rs`, `pub(crate)`-visible to sibling modules:**
- row mappers: `build_user_from_row`, `build_game_bot_from_row`,
  `build_game_type_user`, `build_game_player_from_row`
- pure helpers: `validate_username` (ungated), `normalize_pref_color`,
  `cap_digest`, `active_within_window` (or leave with users),
  `remove_highest_prefs`, `choose_colors`, `LocPref`

**Shared error / plumbing (no change of location needed, they are external):**
- the crate `Result<T>` alias (source module **unverified** - resolve before
  writing the spec)
- `sqlx::PgPool`, `sqlx::PgConnection`, `sqlx::Transaction` - used directly, no
  local alias exists
- `uuid::Uuid`, `time::PrimitiveDateTime`
- `crate::models::user::User`, `crate::models::game::Game`,
  `crate::game::StatusUpdate`

**Row structs used by exactly one module each** (so they move with their code,
not central): `PendingGameRow`, `FinishedGameRow`, `FriendRow`,
`UserEmailRow`, `BotTurn`, `PlayerSlotInternal`.

**Types used by more than one module:** `GameExtended` / `GamePlayerExtended`
(games + email/notify + server_fns), `CreateGameOpts` (game_write + 5 external
files), `StaleStateConflict` (game_write + game/mod.rs). All three are `pub` and
covered by re-export; keep them with their owning module.

**There is no shared transaction helper today.** Every write opens
`pool.begin()` inline. A split does not require inventing one; if the spec wants
one, that is a behaviour change and should be scoped separately.

## G. Mechanics

### Module convention

`ls -d */ ; ls */mod.rs` in `/home/beefsack/Development/brdgme/rust/web/src/`:

| Directory | Has `mod.rs`? | Has sibling `<name>.rs`? |
|---|---|---|
| `auth/` | yes | no |
| `components/` | yes | no |
| `email/` | yes | no |
| `game/` | yes | no |
| `game_info/` | yes | no |
| `models/` | yes | no |
| `stats/` | yes | no |
| `bin/` | n/a (binaries) | n/a |

**`mod.rs` style dominates: 7/7. No `foo.rs` + `foo/` pairs exist.** The split
should produce `db/mod.rs` + `db/*.rs` and delete `db.rs`.

### sqlx offline cache

`/home/beefsack/Development/brdgme/rust/web/.sqlx/` contains 132 json files.
Top-level keys of a cache entry (`jq -r 'keys[]'`):

```
db_name
describe
hash
query
```

There is **no file path, line number, or module field**. The filename is
`query-<sha256-of-query-text>.json` and the `hash` field repeats it.

**Definitive: moving `sqlx::query!` macros between files does NOT invalidate the
offline cache.** `cargo sqlx prepare` is only required if a query's SQL *text*
changes. A pure code move needs no re-prepare.

Caveat: `.sqlx` is per-crate (`rust/web/.sqlx`). All new modules stay inside the
`web` crate, so the cache location is unaffected.

### Tests: private vs public API

Private/`pub(crate)` production items referenced from `mod tests`
(`grep -c '\b<sym>\b'` over the test span):

| Private symbol | Test refs | Binding tests |
|---|---|---|
| `choose_colors` | 6 | `choose_colors_honours_preference`, `choose_colors_same_rank_conflict_resolves_distinctly`, `choose_colors_normalizes_legacy_amber_to_orange`, `choose_colors_normalizes_legacy_bluegrey_to_cyan`, `choose_colors_no_prefs_fills_from_palette_order` |
| `elo_rating_change` | 5 | `elo_rating_change_works`, `elo_rating_change_three_player_pairwise_sums_to_zero`, `find_rating_change` (helper) |
| `apply_rating_changes` | 2 | 1 direct call in a rating test; 1 comment reference |
| `build_game_type_user` | 1 | comment reference only, not a call |
| `build_user_from_row`, `build_game_bot_from_row`, `build_game_player_from_row`, `PendingGameRow`, `FinishedGameRow`, `PlayerSlotInternal`, `remove_highest_prefs`, `LocPref`, `elo_transformed_rating`, `elo_expected_score`, `write_ranked_placings`, `FriendRow`, `normalize_pref_color` | 0 | not referenced |

**Only three private items are directly exercised**: `choose_colors`,
`elo_rating_change`, `apply_rating_changes`. Their tests must move to
`db/common.rs` (colours) and `db/rating.rs` (ELO) respectively. Everything else
in `mod tests` binds to `pub` API and can be placed in whichever module reads
best, or kept in a `db/tests.rs`-style integration module.

Test-to-domain binding (from the 144 test fn names): the module has clear
name-prefixed clusters - `choose_colors_*` (5), `elo_*` / rating / finishing
(~14), `find_public_index_game_id_*` (6), `friend_*` / `block_*` / `unfriend_*`
(~15), `is_game_visible_*` / `game_visibility_*` (~8), `update_game_command_success_*`
(6), `search_users_*` (3), email/`set_primary_*`/`can_*`/`cap_digest_*` (~14),
`delete_game_*` (3), `*_summaries_*` (5), plus ~12 shared fixture helpers
(`make_user`, `make_game_type_and_version`, `make_game_with_players`,
`make_proposal`, `add_proposal_player`, `insert_proposal`, `finish_game`,
`set_recently_active`, `set_stale`, `count_rows`, `position_of`, `check_roster`).

**The fixture helpers are the real coupling risk in the test split** - 12 helpers
are shared across every cluster and need a `db/test_support.rs` (or
`mod tests { mod fixtures; }`) shared module.

### File-level attributes and doc comments to preserve

- Module-level `//!` doc comment, ~55 lines. Contains three things that MUST be
  preserved and distributed:
  1. the `updated_at` trigger convention (which of the 18 tables have a BEFORE
     UPDATE trigger and which need manual `updated_at`) - belongs in `db/mod.rs`
  2. the "Module map" section - **becomes obsolete on split and must be
     rewritten** to describe the new module layout
  3. the `ssr`-gating note, including the explicit carve-out that
     `validate_username` is ungated on purpose - belongs in `db/mod.rs` and must
     be echoed in `db/common.rs`
- `#[allow(clippy::too_many_arguments)]` x3, on `build_game_type_user`,
  `build_game_player_from_row`, and `update_game_command_success`. These are
  item-level, they travel with their function.
- No `#![...]` inner attributes and no file-level `#[allow]`.
- 129 `#[cfg(feature = "ssr")]` item gates. There is **no module-level gate**;
  the split must keep per-item gating (or introduce module-level gates carefully,
  since `validate_username` must remain ungated - putting it in a module with a
  module-level `ssr` gate would break the client build).
- `#[tracing::instrument(...)]` on 12 functions; item-level, travels with code.

## H. Architectural observations (side deliverable - for a LATER review, not this split)

- `db.rs`: the test module is **4838 lines, 59% of the file**. Even after a
  production split, the tests dominate. Consider `web/tests/` integration tests
  for the ones that only touch `pub` API.
- `db.rs::create_game_with_users_tx` at ~205 lines does username generation,
  colour assignment, player insertion, bot insertion and game insertion in one
  body. Prime decomposition candidate.
- `db.rs::apply_rating_changes` at ~151 lines mixes the pure pairwise ELO maths
  with the SQL read/write loop. The pure part is already isolated in `elo_*`;
  the rest could be split into "gather ratings" / "compute" / "persist".
- `db.rs::update_game_command_success` at ~115 lines is the single hottest write
  path and takes enough arguments to need
  `#[allow(clippy::too_many_arguments)]`. A parameter struct is indicated.
- `db.rs::friend_recent_visible_game` duplicates the
  `is_game_visible_to_user` predicate as inlined SQL, guarded only by a test
  asserting the two agree. Genuine duplicated logic; a shared SQL fragment or a
  view would remove the drift risk.
- `db.rs::build_game_type_user` and `build_game_player_from_row` both carry
  `#[allow(clippy::too_many_arguments)]` - row mappers taking positional args
  instead of `sqlx::FromRow`. Three other row types (`PendingGameRow`,
  `FinishedGameRow`, `FriendRow`) do use `#[derive(sqlx::FromRow)]`. Inconsistent.
- `db.rs::cap_digest<T>` is a generic `Vec` truncation with no database
  involvement living in the DB layer. Misplaced responsibility.
- `db.rs::validate_username` is the sole ungated item in an otherwise
  server-only module, purely so the client form can share it. That is a
  shared-validation concern that arguably belongs in `models/` or a small
  shared crate.
- `db.rs::normalize_pref_color` (`pub(crate)`) is consumed by
  `stats/queries.rs` and `email/commands.rs` - a presentation concern reaching
  into the DB layer for a string helper.
- No transaction abstraction anywhere: every write fn calls `pool.begin()`
  inline and hand-rolls commit/rollback. 293 external `db::` call sites all go
  straight to concrete functions - there is no repository/port boundary, so the
  DB layer is not substitutable in tests.
- `db.rs::is_user_recently_active` returns `bool`, not `Result<bool>` - it
  swallows DB errors. Inconsistent with every other query fn.
- Two nearly identical predicate pairs exist purely for the pool-vs-connection
  split: `has_block` / `has_block_conn`, and `find_open_restart_proposal` /
  `find_open_restart_proposal_tx`, plus `create_game_with_users` /
  `create_game_with_users_tx` and `create_game_logs` / `insert_game_logs_tx`.
  A generic `impl Executor` bound would collapse all four pairs.
- `db.rs` reaches back up into `crate::game::server_fns::BotSlot` and
  `crate::game::StatusUpdate` - the data layer depends on the presentation/
  server-fn layer. Inverted dependency; `BotSlot` and `StatusUpdate` look like
  they belong in `models/`.
