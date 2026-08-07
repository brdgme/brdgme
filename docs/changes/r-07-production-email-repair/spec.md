# R-07 Production Email Repair

## Status and Scope

- Owner-approved, one-off production repair. This is not an account-merge tool and creates no reusable application feature.
- The repair preserves only game and proposal participation/ownership. It discards loser ratings, friendships, blocks, chat messages and memberships, authentication tokens and sessions, login confirmations, and loser email rows.
- Do not apply migration 026 or migration 029. Both must remain unapplied after this repair, ready for the owner to apply separately.
- The tested CNPG recovery path is the rollback asset.
- Do not edit any applied migration.

## Approved Mappings

| Group | Survivor user | Retained survivor email row | Loser user | Deleted loser email row |
|---|---|---|---|---|
| 1 | `1aa69b2f-a0f7-4b52-9abb-045426b47481` | `cfa2cca4-54d0-4d2d-b93c-e3a036e41f74` | `faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1` | `580f6f06-f082-465c-b9c4-fa21aef4a6f7` |
| 2 | `4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac` | `76ae8efa-2cd1-4cc4-9fa4-444c1ca723da` | `d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8` | `7024274d-3567-4f05-98b8-dd503290fc0d` |

## Controls

- Use a maintenance window that prevents application writes for the transaction and postchecks. Do not expose credentials, email addresses, token values, session data, or query rows in command output or the transcript; record only counts, row IDs already approved above, and pass/fail evidence in the operator's private record.
- Use bounded `kubectl` snapshots only: no streaming output or unbounded waits. Run no Cargo command except an allowed `cargo check -p web` variant; never run `cargo run`, `cargo test`, `cargo build`, or `cargo clippy` for `web`. Never push.
- The executor is a disposable, reviewed repair script. It binds typed UUIDs and the helper-produced exact old/new email strings. It must not interpolate email values and must never calculate a replacement with PostgreSQL `lower()`.
- Compile the smallest disposable Rust helper outside `rust/web` with `rustc`, not Cargo. For every input email it produces exactly `input.trim().to_lowercase()`. Its input/output stays in an operator-private file. It supplies the exact old/new values for the two approved retained-survivor rows only - see `plan.md` Task 3 for the excluded-singleton rule.
- PostgreSQL `lower(btrim(email))` may be used only for diagnostics and migration-026/029 readiness checks. It is never a source of replacement data.

## Backup and Gates

1. Create CNPG `Backup` `postgres-pre-repair-r07-20260801-01` for Cluster `postgres` in namespace `brdgme`, using plugin backup `barman-cloud.cloudnative-pg.io`.
2. Before any database mutation, require the Backup phase to be `completed` and require the ObjectStore recovery timestamp to advance past this backup. Record the backup completion time and usable recovery timestamp privately. Any backup failure, non-completed phase, or non-advanced timestamp stops the repair.
3. Inspect `_sqlx_migrations`. Require migrations 026 and 029 to each have no applied row. Abort if either is applied, failed, duplicated, or its state is otherwise ambiguous. Do not rerun any migration.
4. Capture a read-only preflight manifest: exact users, the four approved email-row IDs, their ownership, and exactly one email row for each approved user. Any additional email row for those users requires owner approval and otherwise aborts.
5. Capture the direct-FK inventory from `pg_constraint` and require this exact `users` reference set: `user_emails.user_id`, `user_auth_tokens.user_id`, `friends.source_user_id`, `friends.target_user_id`, `blocks.blocker_user_id`, `blocks.blocked_user_id`, `chat_users.user_id`, `game_type_users.user_id`, `game_players.user_id`, `game_proposals.owner_user_id`, and `game_proposal_players.user_id`. Abort on a missing or additional direct reference.
6. Capture the affected game-player IDs/game IDs and proposal-player IDs/proposal IDs/owner IDs. Abort if a survivor and loser are in the same game, if projecting either loser to its survivor duplicates a proposal player, or if that projection makes a proposal owner also a player. Preserve this manifest for postchecks.
7. Use the Rust helper over every `user_emails.email`. Require the only Rust-canonical collision groups to be the two approved survivor/loser pairs; abort on any collision with an unaffected row. A noncanonical row is permitted to exist unresolved by this repair when it is an approved deleted loser row, a retained survivor row (canonicalized), or any other singleton with no collision (left untouched - see `plan.md` Task 3 for the specific excluded row). Separately record the SQL-expression duplicate/noncanonical counts for migration-026/029 readiness.
8. Preflight and the transaction use bounded locking: `SERIALIZABLE`, `lock_timeout = '5s'`, `statement_timeout = '60s'`, and `idle_in_transaction_session_timeout = '60s'`. A timeout or serialization failure is an abort, not a retry against a changed manifest.

## Transaction

Run one transaction only. Re-read and lock the manifest rows with `FOR UPDATE`; every expected count, row ID, owner, and old email value must match before its mutation. Any zero, extra, or unexpected affected-row count raises an error and rolls back the whole transaction.

1. Revoke loser authentication: delete loser `user_auth_tokens`; invalidate login confirmations for the bound old and canonical email values. Do not attempt to clear settings/unsubscribe token columns: this repair runs against production's schema as of migration 022 (see `plan.md` Task 4 precondition), before those columns exist, and they are moot regardless since the loser `users` row is deleted outright in step 7.
2. Discard loser sessions. `tower_sessions.session.data` is opaque MessagePack, not a foreign key: decode records through the existing session-store format, select only records whose decoded `SessionUser.id` is a loser, assert the selected count against the preflight manifest, then delete those session IDs. Do not byte-match opaque session data or delete unrelated sessions.
3. Transfer `game_players` from each loser to its survivor and set every transferred `email_token` to `NULL`. Assert the exact preflight player-ID set and count.
4. Transfer `game_proposal_players` from each loser to its survivor and set every transferred `email_token` to `NULL`. Transfer `game_proposals.owner_user_id` from each loser to its survivor. Assert the exact preflight proposal row and owner-ID sets and counts.
5. Delete approved ancillary data in FK order: `chat_messages` through the loser `chat_users`, then loser `chat_users`; loser `friends` and `blocks` in either endpoint; and loser `game_type_users`. Do not alter games, game logs, game-log targets, chats, or unrelated rows.
6. Delete exactly the two approved loser `user_emails` rows. Update only the two retained survivor rows with their bound Rust-helper canonical value. No other row is touched - see `plan.md` Task 3 for the deliberately excluded singleton. Assert each old value and affected row ID; no SQL expression derives the new value.
7. Delete exactly the two loser `users` rows. The direct-FK inventory must now be fully resolved. Commit only after all assertions pass.

## Postchecks

- Verify neither loser user exists and no remaining direct-FK reference, decoded session, or login-confirmation email value refers to either loser.
- Verify the preflight game-player IDs/game IDs and proposal-player IDs/proposal IDs are unchanged in count and identity; transferred rows point to the approved survivor. Verify no game or proposal row was created, removed, or otherwise changed outside the specified ownership fields and cleared email tokens.
- Re-run the Rust helper across all stored emails: zero Rust-canonical duplicate groups, and noncanonical rows limited to exactly the one deliberately excluded singleton (see `plan.md` Task 3) - it remains unresolved until migration 026 runs. Re-run SQL-expression diagnostics: `email <> lower(btrim(email))` limited to that same one row, and zero `lower(btrim(email))` duplicate groups. Report counts only.
- Re-inspect `_sqlx_migrations`: 026 and 029 remain unapplied. Confirm the table now satisfies migration 026's duplicate guard and migration 029's check/index prerequisites, but do not apply either.
- Mark R-07 complete in the tracker only after every postcheck passes. Do not edit the tracker as part of this documentation change. Then proceed to R-08.

## Failure Handling

- Before commit: any gate, assertion, lock, timeout, serialization, or SQL failure rolls back the transaction and stops. Do not retry automatically.
- After commit: any failed postcheck stops further writes and escalation proceeds to the owner with the private evidence. Decide whether to recover with CNPG/PITR to the verified pre-repair recovery timestamp; do not attempt ad hoc compensating edits.
- Backup failure always stops before database mutation; CNPG recovery is the only rollback asset.
