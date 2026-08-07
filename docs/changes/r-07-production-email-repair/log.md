# Log: R-07 Production Email Repair

Append-only. Append after a meaningful checkpoint: completed unit, verified
partial result, consequential discovery, blocker, commit, or approved plan
change. Each entry records timestamp, role and work unit, result, changed files
or commit, verification, and any discovery, blocker, or required decision. No
narration, copied output, or speculation.

## 2026-08-07 - Lead - unit-01 (production dump, local restore, full pending-migration batch test)

Result: accepted. Investigation-only; no repair migration written, no
`spec.md`/`plan.md`/migration files touched.

Method: one Worker dispatch (depth-two, confirmed working) performed a
read-only `pg_dump` of production `brdgme` (user-authorised, exact command
per brief; excluded `user_auth_tokens`/`session` row data) to
`/tmp/opencode/r07-production-email-repair/brdgme-prod-20260807.dump`
(31,007,772 bytes), restored it inside the local docker-compose `postgres:18`
container into scratch DB `r07_migration_test` (not the dev `brdgme` DB), and
ran `sqlx migrate run` (host sqlx-cli 0.9.0 against `127.0.0.1:5432`) for the
full pending batch. Lead independently verified: `git status --short` clean
(no PII entered the repo), dump file present at reported size, compose
`postgres` service healthy, and `r07_migration_test._sqlx_migrations` shows
001-025 committed with `success=t` - matches the Worker's report exactly.

Findings:
- Applied-in-production (from dump's own `_sqlx_migrations`): 001-022.
  Pending set: 023-032.
- Test run applied 023, 024, 025 successfully, then **026
  (`026_canonical_emails.sql`) failed first**, not 029 as the existing
  spec/plan assumed. 027-032 not reached. Root cause independently confirmed
  by reading `026_canonical_emails.sql:17-27`: a `DO` block groups
  `user_emails` by `lower(btrim(email))` and `RAISE EXCEPTION`s on any
  duplicate group before the canonicalizing `UPDATE` and its own
  `user_emails_email_lower_key` unique index. `029_canonical_email_check.sql`
  (read to confirm) explicitly builds on 026 (drops
  `user_emails_email_lower_key`, adds the CHECK + canonical index) - 029
  cannot even be reached until 026 clears, and 026 is gated by the exact
  same two duplicate-email groups R-07 already targets. No new repair scope:
  the approved repair (canonicalizing the same two groups via
  `lower(btrim(email))`/Rust `trim().to_lowercase()`) resolves both guards
  identically. This is a plan-accuracy gap (spec/plan mention only 029) for
  the docs-correction unit to fix, not a scope change.
- No other migration failure in 023-032. No sqlx metadata/schema-mismatch
  error.
- Duplicate-email-group shape in restored production data: exactly 2 groups
  of 2 rows each, IDs matching the approved mapping table exactly
  (`cfa2cca4-54d0-4d2d-b93c-e3a036e41f74`/`580f6f06-f082-465c-b9c4-fa21aef4a6f7`
  and
  `76ae8efa-2cd1-4cc4-9fa4-444c1ca723da`/`7024274d-3567-4f05-98b8-dd503290fc0d`).
  No unapproved/additional duplicates found. `public.users` row count in
  restored copy: 14.
- Disk: `/tmp` 226G free/51% used after the dump; not a concern.

Artifacts left in place for the next (repair-implementation) unit, all
outside the repo and outside git:
- Dump: `/tmp/opencode/r07-production-email-repair/brdgme-prod-20260807.dump`
  (also copied in-container at `postgres:/tmp/brdgme-prod.dump`).
- Scratch DB `r07_migration_test` in the running `brdgme-dev-postgres-1`
  compose container (`postgres://brdgme_user:brdgme_password@127.0.0.1:5432/r07_migration_test`),
  currently at migration version 25 (23-25 were committed by this test, so it
  is one step past the raw dump state - noted for whoever reuses it).

No PII (email values) entered this log, the transcript summary, or any
committed file; the one PII-bearing artifact (the dump) stays under
`/tmp/opencode/`.

## 2026-08-07 - Lead - unit-02, approved plan.md/spec.md amendment (pre-implementation)

Result: `spec.md`/`plan.md` edited with Orchestrator/user approval, not yet
committed (bundled with unit-02's eventual completion commit).

Trigger 1 (user pre-authorised, narrow scope): unit-01 found migration 026
fails before 029 is ever reached, on the same two duplicate-email groups.
Added "026" alongside every existing "029" mention in `spec.md`/`plan.md`
that discusses migration readiness, the guard, or apply-ordering (global
constraints, Task 5 title/interfaces/SQL/expected-output/Step 3, Task 6
tracker text, inline self-review, and the equivalent spec.md Status/Scope,
Controls, Backup-and-Gates, and Postchecks clauses). No other semantic
change; acceptance criteria, transaction design, and table scope untouched.

Trigger 2 (Worker-discovered defect, Orchestrator ruling obtained before any
edit): the Task 2 Step 2 locked preflight opened
`BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY;` and used
`... FOR SHARE` inside two CTEs. PostgreSQL rejects `FOR SHARE`/`FOR UPDATE`
inside any `READ ONLY` transaction unconditionally - confirmed against local
Postgres 18.4, not version-specific, so this would have failed identically
against real production. Ruling obtained: keep `READ ONLY` (it is the only
mechanical guard against a hand-run production psql session writing),
remove the two `FOR SHARE` clauses instead (locks taken inside a block that
always `ROLLBACK`s protect nothing forward into the later mutating
transaction, which already takes its own `FOR UPDATE` locks as its literal
first step - confirmed by direct read of `plan.md:436` before ruling out a
locking gap there). Collapsed both CTEs to plain `count(*) ... WHERE id IN
(...)` queries preserving the exact UUID lists and output label strings;
line 263's expected-output list updated to match. No other line in the
preflight, or in Task 2 Step 3's FK-inventory query, or in Task 5's
postcheck (neither uses row locks) required this fix.

Verified after editing: `grep -n "FOR SHARE"` across `plan.md`/`spec.md`
returns no matches; all `migration_029`/`migration-029` mentions now have a
paired `migration_026`/`migration-026` counterpart.

## 2026-08-07 - Lead - unit-02, two further mechanical SQL fixes (reserved-word aliases)

Result: `plan.md` edited, not yet committed. Applied directly without
escalation - these are unambiguous PostgreSQL reserved-keyword syntax
errors with a single correct fix (bare identifier rename), not design
tradeoffs, unlike the `FOR SHARE`/`READ ONLY` case above.

- `plan.md:232` (Task 2 preflight, `same_group_game_overlap` subquery):
  bare alias `overlaps` is a PostgreSQL reserved keyword
  (`ERROR: syntax error at or near "overlaps"`, hit by the Worker running
  against local Postgres 18.4). Renamed alias to `game_overlaps`.
- `plan.md:488` (Task 5 postcheck, `remaining_direct_fk_references`
  subquery): bare alias `references` is also a PostgreSQL reserved keyword.
  Found by Lead pre-emptively (tested `SELECT 1 AS x FROM (SELECT 1)
  references;` locally, confirmed same error) before the Worker reached
  Task 5, to avoid a third round-trip. Renamed alias to `ref_counts`.

Verified: `grep -n ") [a-z_]*;$"` across the file now shows only
`one_email`, `game_overlaps`, `duplicate_groups` (x2), `ref_counts` - all
confirmed non-reserved via local `psql` test. Both fixes preserve exact
column/output semantics; no count, condition, or table reference changed.

## 2026-08-07 - Lead - unit-02, three Orchestrator/user rulings applied to plan.md/spec.md

Worker checkpoint 1 (helper crate built, not yet run) surfaced two genuine
ambiguities; escalated; user ruled on both plus a third item the escalation
exposed. All three applied directly (mechanical text edits, no further
judgment required once ruled):

- **Ruling 1** (singleton canonicalization scope): confirmed narrow reading
  - only actually-noncanonical singleton rows are canonicalized, not every
  singleton. `plan.md`'s Task 3 email-algorithm contract reworded "every
  singleton" -> "every noncanonical singleton", with an explicit
  "already-canonical singleton rows are not touched" sentence added.
  `spec.md`'s Transaction step 6 reworded "each preflight-approved singleton
  row" -> "each preflight-approved noncanonical singleton row" for
  consistency.
- **Ruling 2** (token-clearing clause unexecutable at production's actual
  schema): removed "clear loser `users.settings_email_token`,
  `settings_token_expires_at`, `settings_token_used_at`, and
  `unsubscribe_token` before user deletion" from `plan.md` Task 4 Step 2 and
  from `spec.md` Transaction step 1. Replaced with an explanatory sentence:
  those columns originate in migrations 023/025/027, none applied at repair
  time (production is at 022), and are moot regardless since the loser
  `users` row is deleted outright in step 7.
- **Ruling 3** (execution ordering, previously unstated and load-bearing):
  added an explicit "Precondition (execution ordering, load-bearing)" note
  to `plan.md` immediately before Task 4 Step 1's fixed sequence: the repair
  transaction runs against production at its current schema (022) BEFORE any
  further migration is applied; the full pending batch 023-032 then runs
  afterward, uninterrupted, in one migration window. This matches what the
  local test (`r07_repair_test`, restored directly from the dump, untouched
  by any migration before the repair) already reproduces - no change needed
  to the test's own sequencing.

A schema sweep of the entire plan's mutating/verification steps against the
actual migration-022 schema (via `information_schema` on `r07_repair_test`,
not the migration files) found no further gaps: `game_players.email_token`/
`game_proposal_players.email_token` exist and are nullable;
`login_confirmations` has no `user_id` column (email is its PK, matching by
email value is correct); `chat_messages.chat_user_id -> chat_users.id ->
users.id` is the correct join for the FK-ordered deletion;
`tower_sessions.session` has exactly the three columns the helper's decoder
assumes. `FOR UPDATE` locking on the four approved rows is confirmed present
as the first two statements of the generated transaction, before any
assertion (Postgres rejects `FOR UPDATE` combined with an aggregate, so the
lock and the count-assertions are separate statements rather than one query
- same effective lock, no plan-intent change).

## 2026-08-07 - Lead - unit-02, checkpoint-2 finding: an out-of-mapping singleton row, and ruling

At Checkpoint 2 (helper run, `repair.sql` generated but not yet executed),
the helper's manifest showed `singleton_rows_to_canonicalize: 1` - a row
outside the approved 4-row mapping entirely: `user_emails.id =
'0ad09a53-ea45-495e-ae46-820245f2bcbb'`, owner `user_id =
'0b9208e6-6bd9-435b-b352-8d4dc3cff3e4'`. Escalated before running anything
mutating. User requested a bounded, read-only fact-finding pass (no
production access, no mutation, full email-redaction discipline) before
ruling. Findings, evidence-based:

- The row's canonical form collides with no other row (true singleton, not a
  hidden third duplicate).
- Its 69-character stored value is not an email address: sentence-like free
  text (including a U+00A0 non-breaking space and a `!`) with one
  syntactically well-formed email-shaped substring at the very end.
  `lower(btrim())` of the full value does not produce a valid single email
  address - it only lowercases the surrounding sentence text.
  Neither migration 026's duplicate guard nor 029's CHECK constraint is
  affected by this row's content either way (026's guard only fires on
  collisions; 026's own blanket `UPDATE ... WHERE email <> lower(btrim(email))`
  independently normalizes every noncanonical row, including this one, when
  026 itself runs - so 029's later CHECK holds for it regardless of whether
  R-07 touches it).
- The embedded address-shaped substring, after `lower(btrim())`, exactly
  matches Group 2's approved pair - both
  `76ae8efa-2cd1-4cc4-9fa4-444c1ca723da` (Group 2 survivor,
  `4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac`) and
  `7024274d-3567-4f05-98b8-dd503290fc0d` (Group 2 loser,
  `d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8`), which are each their own
  account's only `user_emails` row.
- The singleton's own owner (`0b9208e6-...`) has 1 `game_players` row and 0
  `game_proposals` as owner - lightly used.

**Ruling (user): option (b), exclude.** R-07 canonicalizes only the two
approved retained-survivor rows; this row and any other out-of-mapping
noncanonical singleton are left untouched, deliberately, because migration
026's own blanket update will canonicalize them anyway when 026 runs -
R-07 touching it would be redundant mutation outside the approved 2-account
blast radius. This supersedes (not merely narrows) the earlier "every
noncanonical singleton" rule from the first checkpoint - that generic rule
is gone.

Applied to `plan.md`/`spec.md` (not yet committed): Task 3's email algorithm
now says R-07 canonicalizes only the two named retained-survivor email-row
IDs, explicitly names and excludes `0ad09a53-ea45-495e-ae46-820245f2bcbb`
with the one-line reason above, and states real duplicate collisions outside
the mapping are still an abort (unchanged safety property - only the
no-collision-singleton handling changed). Task 4 Step 6, Task 5's postcheck
expected values, Task 5 Step 2/3's expected helper/SQL-diagnostic output,
the Architecture summary, and the tracker-completion text all updated to
match: post-repair `sql_noncanonical=1` (not 0) is now the correct expected
postcheck value, and the Rust helper is expected to report exactly one
noncanonical row post-repair, both referring to this same excluded row,
until migration 026 later normalizes it.

## 2026-08-07 - Lead - backlog findings (recorded only, not investigated or fixed, per user instruction)

Out of scope for R-07; the user is raising both with the wider team.
Recorded here for a later unit to file as backlog entries. Row/account IDs
only, no email content, per standing PII discipline.

1. **Missing/broken email-address input validation.** Production accepted a
   69-character sentence (containing `!` and a U+00A0 non-breaking space) as
   the value of `user_emails.email` for row `0ad09a53-ea45-495e-ae46-820245f2bcbb`
   (owner `0b9208e6-6bd9-435b-b352-8d4dc3cff3e4`). Whatever write path
   produced this row performed no email-shape validation. Entry point not
   investigated - out of scope here.
2. **Account `0b9208e6-6bd9-435b-b352-8d4dc3cff3e4` is likely non-functional.**
   Its only `user_emails` row is unusable free text, so it probably cannot
   receive mail or use email-based login/settings flows today. After
   migration 026 eventually runs, the value becomes a lowercased version of
   the same sentence - still not a usable address. The account is lightly
   used (1 `game_players` row, 0 `game_proposals` as owner) but not
   necessarily abandoned. Needs owner review; explicitly out of R-07's scope
   per the ruling above.
3. **Observation, not a conclusion:** the garbage text's embedded address
   substring is not arbitrary - it is verbatim Group 2's exact approved
   email address (both survivor and loser forms match). This raises the
   possibility of a shared root cause between this anomaly and the Group 2
   duplicate-account situation R-07 is repairing, rather than two
   independent incidents. Not investigated further here.
