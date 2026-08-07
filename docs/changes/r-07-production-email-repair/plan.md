# R-07 Production Email Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair the two owner-approved duplicate-email account groups in production while preserving every game and proposal, then record verified completion of R-07.

**Architecture:** This is a one-off operational repair, not an application change. A verified CNPG plugin backup precedes a read-only, locked preflight and a disposable local helper produces a private, fully-bound SQL transaction from that preflight. The transaction transfers only game/proposal ownership and participation, removes the approved losing-account ancillary data, and canonicalizes the two approved retained-survivor email rows using Rust semantics. Noncanonical rows outside the approved mapping (including singletons with no collision) are out of scope and are left for migration 026's own blanket canonicalization when it later runs.

**Tech Stack:** Kubernetes, CloudNativePG Backup plugin, PostgreSQL/psql, a disposable standalone Rust crate, git.

## Global Constraints

- Only Workers execute commands; Leads coordinate and review. Delegate serially.
- Never push.
- Never use `git add -A` or `git add .`; stage the exact named file only.
- Do not touch unrelated or untracked files, especially `docs/reviews/2026-07-30-review-session/R-07-HANDOVER.md`.
- Use bounded `kubectl` snapshots and bounded polling only. Do not stream logs or use an unbounded wait.
- Never print, decode into the transcript, or commit secrets, credentials, tokens, connection strings, emails, names, session payloads, or other row-level PII. Report only counts, approved UUIDs, and pass/fail states.
- **Deliberate deviation, recorded here so it is not rediscovered:** every `kubectl exec ... -- psql ... < file.sql` invocation (Tasks 2 and 4) needs `exec -i` for the shell's `< file` stdin redirection to actually reach psql - without `-i`, `kubectl exec` doesn't attach stdin, psql reads nothing, and the command exits 0 having executed no SQL at all. Any future stdin-redirected psql invocation (e.g. Task 5's postcheck, which reuses this pattern) must include `-i` too.
- **Deliberate deviation, recorded here so it is not rediscovered:** every `psql` invocation in Tasks 2, 4, and 5 connects as `-U postgres`, not `-U brdgme_user` as this plan originally assumed. `kubectl exec -c postgres` runs as OS user `postgres`, and this cluster's `pg_hba.conf` only allows peer auth on that local socket, mapped solely to DB role `postgres` - there is no working auth path to `brdgme_user` without reading its password Secret over TCP. Owner ruling: use the existing peer-auth `postgres` path (already the access pattern used for this session's earlier production `pg_dump`) rather than read a Secret into a Worker's context. The repair SQL is hardcoded to 4 approved UUIDs with fail-closed assertions throughout and is personally inspected by the Lead before execution regardless of which role runs it, so the privilege difference between `postgres` and `brdgme_user` does not change the operation's actual safety.
- CNPG Backup is the only rollback asset for the production repair; `pg_dump` output is never a substitute for it.
- A read-only `pg_dump` for local migration-batch verification is permitted only under explicit user authorization (as performed for the `c0275c7c` verification).
- Do not apply migration 026 or migration 029. Neither may have a row in `public._sqlx_migrations` before or after the repair.
- Do not edit any migration under `rust/web/migrations/`.
- Do not run `scripts/rust-test.sh`, workspace-wide Cargo, or workspace-wide `rustc`.
- For `web`, only `cargo check` variants are permitted. Do not use `cargo build`, `cargo test`, `cargo run`, `cargo clippy`, or `rustc` for `web`.
- The standalone helper is outside the workspace and targets one disposable crate only. Its canonicalization function is exactly `input.trim().to_lowercase()`.
- SQL `lower(btrim(email))` is diagnostic-only. Never use it to derive a replacement email value.
- Use exactly these mappings:

| Group | Survivor user | Retained email row | Loser user | Deleted email row |
|---|---|---|---|---|
| 1 | `1aa69b2f-a0f7-4b52-9abb-045426b47481` | `cfa2cca4-54d0-4d2d-b93c-e3a036e41f74` | `faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1` | `580f6f06-f082-465c-b9c4-fa21aef4a6f7` |
| 2 | `4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac` | `76ae8efa-2cd1-4cc4-9fa4-444c1ca723da` | `d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8` | `7024274d-3567-4f05-98b8-dd503290fc0d` |

---

## Files

- Create, ephemeral and private: `/tmp/opencode/r07-production-email-repair/backup.yaml`
- Create, ephemeral and private: `/tmp/opencode/r07-production-email-repair/preflight.sql`
- Create, ephemeral and private: `/tmp/opencode/r07-production-email-repair/r07-repair-helper/`
- Create, ephemeral and private: `/tmp/opencode/r07-production-email-repair/repair.sql`
- Modify after all production postchecks pass: `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md`
- Create in this task: `docs/changes/r-07-production-email-repair/plan.md`

### Task 1: Create And Verify The CNPG Recovery Point

**Files:**
- Create: `/tmp/opencode/r07-production-email-repair/backup.yaml`
- Modify: none
- Test: bounded Backup and Cluster status polling

**Interfaces:**
- Consumes: production namespace `brdgme`, Cluster `postgres`, plugin `barman-cloud.cloudnative-pg.io`.
- Produces: a completed Backup with a fresh UTC-timestamped name (generated at run time, never hardcoded - see Step 2) and a recorded recovery window covering it, confirmed via the `ObjectStore` resource (see Step 4 note on why not `Cluster.status`).

- [ ] **Step 1: Confirm the production objects without reading Secrets**

```bash
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get cluster postgres --namespace=brdgme -o jsonpath='{.metadata.name}{"\n"}{.status.currentPrimary}{"\n"}'
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get objectstore postgres-backup --namespace=brdgme -o jsonpath='{.metadata.name}{"\n"}'
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get pods --namespace=brdgme -l cnpg.io/cluster=postgres -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.status.phase}{"\n"}{end}'
```

Expected: cluster `postgres`, ObjectStore `postgres-backup`, and exactly one running CNPG instance pod. Abort before creating the Backup if any object is absent, more than one instance pod is selected, or the instance pod is not `Running`.

- [ ] **Step 2: Generate a fresh timestamped name, write and apply the one-off Backup CR**

The Backup name is generated fresh every run, never hardcoded. A hardcoded name lets a retry silently no-op against a stale prior Backup (idempotent `kubectl apply` returns `configured`, not `created`) while the operator believes a fresh recovery point was taken - exactly the failure mode that makes the rollback guarantee worthless when it's actually needed. Persist the generated name to a file so later steps (and later Tasks, if resumed in a new shell) read the same value:

```bash
mkdir -p /tmp/opencode/r07-production-email-repair
BACKUP_NAME="postgres-pre-repair-r07-$(date -u +%Y%m%dT%H%M%SZ)"
printf '%s' "$BACKUP_NAME" > /tmp/opencode/r07-production-email-repair/backup-name.txt
cat > /tmp/opencode/r07-production-email-repair/backup.yaml <<EOF
apiVersion: postgresql.cnpg.io/v1
kind: Backup
metadata:
  name: ${BACKUP_NAME}
  namespace: brdgme
spec:
  cluster:
    name: postgres
  method: plugin
  pluginConfiguration:
    name: barman-cloud.cloudnative-pg.io
EOF
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml apply --namespace=brdgme -f /tmp/opencode/r07-production-email-repair/backup.yaml
```

Expected: `backup.postgresql.cnpg.io/<BACKUP_NAME> created`. Because the name is freshly timestamped, `created` is the only acceptable result - `configured` (or any other non-`created` result) means an unexpected name collision; abort rather than proceeding on a Backup object that might not be the one this run just requested.

- [ ] **Step 3: Poll the Backup phase with a five-minute bound**

```bash
BACKUP_NAME=$(cat /tmp/opencode/r07-production-email-repair/backup-name.txt)
for attempt in $(seq 1 10); do
  phase=$(kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get backup "$BACKUP_NAME" --namespace=brdgme -o jsonpath='{.status.phase}' 2>/dev/null)
  case "$phase" in
    completed) printf '%s\n' 'PASS: Backup completed'; break ;;
    failed) printf '%s\n' 'ABORT: Backup failed'; exit 1 ;;
    '') printf '%s\n' "attempt $attempt: phase pending" ;;
    *) printf '%s\n' "attempt $attempt: phase $phase" ;;
  esac
  if [ "$attempt" -eq 10 ]; then
    printf '%s\n' 'ABORT: Backup did not complete within 300 seconds'
    exit 1
  fi
  sleep 30
done
```

Expected: only phase/status output and final `PASS: Backup completed`. Abort on `failed`, a kubectl error, or expiry of the ten-attempt bound.

- [ ] **Step 4: Confirm the recovery point advanced with a five-minute bound**

`Cluster.status.firstRecoverabilityPoint` (and the sibling `lastSuccessfulBackup`/`lastFailedBackup`/`*ByMethod` fields) are CNPG's legacy in-tree-backup fields. As of CNPG 1.30.0 they are documented `Deprecated: the field is not set for backup plugins` and are never populated when using plugin-based backup (`barman-cloud.cloudnative-pg.io`, as this cluster does) - confirmed empirically on this cluster across 32 days and 34 successful backups, and confirmed against the CNPG 1.30.0 CRD, release notes, and plugin-barman-cloud 0.13.0 docs. For plugin-based backup, the equivalent, continuously-maintained state lives on the `ObjectStore` resource instead, at `status.serverRecoveryWindow.<cluster-name>`. Do not check `Cluster.status` here - check `ObjectStore.status.serverRecoveryWindow.postgres` instead:

```bash
BACKUP_NAME=$(cat /tmp/opencode/r07-production-email-repair/backup-name.txt)
for attempt in $(seq 1 10); do
  backup_time=$(kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get backup "$BACKUP_NAME" --namespace=brdgme -o jsonpath='{.status.startedAt}' 2>/dev/null)
  recovery_time=$(kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get objectstore postgres-backup --namespace=brdgme -o jsonpath='{.status.serverRecoveryWindow.postgres.lastSuccessfulBackupTime}' 2>/dev/null)
  if [ -n "$backup_time" ] && [ -n "$recovery_time" ] && [ "$recovery_time" ">" "$backup_time" ]; then
    printf '%s\n' 'PASS: recoverability point is later than Backup start'
    break
  fi
  if [ "$attempt" -eq 10 ]; then
    printf '%s\n' 'ABORT: recoverability point did not advance within 300 seconds'
    exit 1
  fi
  sleep 30
done
```

Expected: `PASS: recoverability point is later than Backup start`. Record the two timestamps in the operator's private incident evidence, not in the transcript. Abort before any database mutation if the status field is absent or the bound expires.

### Task 2: Capture A Locked Read-Only Repair Manifest And Dry Run

**Files:**
- Create: `/tmp/opencode/r07-production-email-repair/preflight.sql`
- Create: `/tmp/opencode/r07-production-email-repair/private/` with mode `0700`
- Modify: none
- Test: one read-only serializable psql session that rolls back

**Interfaces:**
- Consumes: completed Task 1 Backup and the four approved mappings.
- Produces: private TSV snapshots for the disposable helper, a count-only manifest, and an explicit no-write dry-run decision.

- [ ] **Step 1: Select the only running CNPG instance pod and create private storage**

```bash
PGPOD=$(kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get pods --namespace=brdgme -l cnpg.io/cluster=postgres -o jsonpath='{range .items[?(@.status.phase=="Running")]}{.metadata.name}{"\n"}{end}')
test "$(printf '%s\n' "$PGPOD" | wc -l)" -eq 1
mkdir -p /tmp/opencode/r07-production-email-repair/private
chmod 700 /tmp/opencode/r07-production-email-repair/private
```

Expected: one pod name is retained in `PGPOD`; no private data is displayed. Abort if the selected line count is not one.

- [ ] **Step 2: Run the locked read-only preflight and collect only count-safe output**

Write `/tmp/opencode/r07-production-email-repair/preflight.sql` with this SQL. The `COPY` commands are intentionally separate private exports, run after the count-only transaction; their output is redirected to private files and never displayed.

```sql
\set ON_ERROR_STOP on
BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY;
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '60s';
SET LOCAL idle_in_transaction_session_timeout = '60s';

SELECT 'migration_026_rows=' || count(*)
FROM public._sqlx_migrations
WHERE version = 26;

SELECT 'migration_029_rows=' || count(*)
FROM public._sqlx_migrations
WHERE version = 29;

SELECT 'approved_users=' || count(*)
FROM public.users
WHERE id IN (
  '1aa69b2f-a0f7-4b52-9abb-045426b47481',
  'faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1',
  '4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac',
  'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8'
);

SELECT 'approved_email_rows=' || count(*)
FROM public.user_emails
WHERE id IN (
  'cfa2cca4-54d0-4d2d-b93c-e3a036e41f74',
  '580f6f06-f082-465c-b9c4-fa21aef4a6f7',
  '76ae8efa-2cd1-4cc4-9fa4-444c1ca723da',
  '7024274d-3567-4f05-98b8-dd503290fc0d'
);

SELECT 'email_owner_mapping_ok=' || count(*)
FROM public.user_emails
WHERE (id, user_id) IN (
  ('cfa2cca4-54d0-4d2d-b93c-e3a036e41f74'::uuid, '1aa69b2f-a0f7-4b52-9abb-045426b47481'::uuid),
  ('580f6f06-f082-465c-b9c4-fa21aef4a6f7'::uuid, 'faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1'::uuid),
  ('76ae8efa-2cd1-4cc4-9fa4-444c1ca723da'::uuid, '4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac'::uuid),
  ('7024274d-3567-4f05-98b8-dd503290fc0d'::uuid, 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8'::uuid)
);

SELECT 'one_email_per_approved_user=' || count(*)
FROM (
  SELECT user_id
  FROM public.user_emails
  WHERE user_id IN (
    '1aa69b2f-a0f7-4b52-9abb-045426b47481', 'faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1',
    '4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8'
  )
  GROUP BY user_id
  HAVING count(*) = 1
) one_email;

SELECT 'direct_user_fks=' || count(*)
FROM pg_constraint c
JOIN pg_class child ON child.oid = c.conrelid
JOIN pg_namespace child_ns ON child_ns.oid = child.relnamespace
JOIN pg_class parent ON parent.oid = c.confrelid
JOIN pg_namespace parent_ns ON parent_ns.oid = parent.relnamespace
WHERE c.contype = 'f'
  AND child_ns.nspname = 'public'
  AND parent_ns.nspname = 'public'
  AND parent.relname = 'users';

SELECT 'same_group_game_overlap=' || count(*)
FROM (
  SELECT game_id FROM public.game_players
  WHERE user_id IN ('1aa69b2f-a0f7-4b52-9abb-045426b47481', 'faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1')
  GROUP BY game_id HAVING count(DISTINCT user_id) = 2
  UNION ALL
  SELECT game_id FROM public.game_players
  WHERE user_id IN ('4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  GROUP BY game_id HAVING count(DISTINCT user_id) = 2
) game_overlaps;

SELECT 'proposal_player_collision=' || count(*)
FROM public.game_proposal_players loser
JOIN public.game_proposal_players survivor
  ON survivor.proposal_id = loser.proposal_id
 AND ((loser.user_id = 'faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1' AND survivor.user_id = '1aa69b2f-a0f7-4b52-9abb-045426b47481')
   OR (loser.user_id = 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8' AND survivor.user_id = '4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac'));

SELECT 'owner_player_collision=' || count(*)
FROM public.game_proposals p
JOIN public.game_proposal_players pp
  ON pp.proposal_id = p.id
 AND ((p.owner_user_id = 'faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1' AND pp.user_id = '1aa69b2f-a0f7-4b52-9abb-045426b47481')
   OR (p.owner_user_id = 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8' AND pp.user_id = '4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac'));

SELECT 'games_total=' || count(*) FROM public.games;
SELECT 'proposals_total=' || count(*) FROM public.game_proposals;
SELECT 'sql_noncanonical=' || count(*) FROM public.user_emails WHERE email <> lower(btrim(email));
SELECT 'sql_duplicate_groups=' || count(*) FROM (
  SELECT lower(btrim(email)) FROM public.user_emails GROUP BY 1 HAVING count(*) > 1
) duplicate_groups;
ROLLBACK;
```

Run:

```bash
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec -i --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -qAt -v ON_ERROR_STOP=1 -U postgres -d brdgme < /tmp/opencode/r07-production-email-repair/preflight.sql
```

Expected safe output: `migration_026_rows=0`, `migration_029_rows=0`, `approved_users=4`, `approved_email_rows=4`, `email_owner_mapping_ok=4`, `one_email_per_approved_user=4`, `direct_user_fks=11`, `same_group_game_overlap=0`, `proposal_player_collision=0`, `owner_player_collision=0`, plus count-only game/proposal and SQL readiness values. Abort on a timeout, serialization error, nonzero psql exit, any different required count, or any additional email row for an approved user.

- [ ] **Step 3: Verify the exact direct-FK inventory rather than relying on its count**

```sql
SELECT child.relname || '.' || child_att.attname
FROM pg_constraint c
JOIN pg_class child ON child.oid = c.conrelid
JOIN pg_namespace child_ns ON child_ns.oid = child.relnamespace
JOIN pg_class parent ON parent.oid = c.confrelid
JOIN pg_namespace parent_ns ON parent_ns.oid = parent.relnamespace
JOIN unnest(c.conkey) WITH ORDINALITY AS child_key(attnum, ord) ON true
JOIN pg_attribute child_att ON child_att.attrelid = child.oid AND child_att.attnum = child_key.attnum
WHERE c.contype = 'f'
  AND child_ns.nspname = 'public'
  AND parent_ns.nspname = 'public'
  AND parent.relname = 'users'
ORDER BY 1;
```

Run the query in a separate `BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY` session with the same local timeouts and `ROLLBACK`. Expected output, exactly and in this order: `blocks.blocked_user_id`, `blocks.blocker_user_id`, `chat_users.user_id`, `friends.source_user_id`, `friends.target_user_id`, `game_players.user_id`, `game_proposal_players.user_id`, `game_proposals.owner_user_id`, `game_type_users.user_id`, `user_auth_tokens.user_id`, `user_emails.user_id`. Abort on any missing or extra line.

- [ ] **Step 4: Export private, complete input snapshots and perform the non-mutating topology dry run**

```bash
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -qAt -F $'\t' -U postgres -d brdgme \
  -c "SELECT id, user_id, email FROM public.user_emails ORDER BY id" \
  > /tmp/opencode/r07-production-email-repair/private/emails.tsv
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -qAt -F $'\t' -U postgres -d brdgme \
  -c "SELECT id, encode(data, 'hex') FROM tower_sessions.session ORDER BY id" \
  > /tmp/opencode/r07-production-email-repair/private/sessions.tsv
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -qAt -F $'\t' -U postgres -d brdgme \
  -c "SELECT id, game_id, user_id, email_token IS NOT NULL FROM public.game_players WHERE user_id IN ('1aa69b2f-a0f7-4b52-9abb-045426b47481','faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1','4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac','d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8') ORDER BY id" \
  > /tmp/opencode/r07-production-email-repair/private/game-players.tsv
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -qAt -F $'\t' -U postgres -d brdgme \
  -c "SELECT id, proposal_id, user_id, email_token IS NOT NULL FROM public.game_proposal_players WHERE user_id IN ('1aa69b2f-a0f7-4b52-9abb-045426b47481','faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1','4e7f9c6b-a0fc-4d6c-847f-08e2c4e4baac','d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8') ORDER BY id" \
  > /tmp/opencode/r07-production-email-repair/private/proposal-players.tsv
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -qAt -F $'\t' -U postgres -d brdgme \
  -c "SELECT id, owner_user_id FROM public.game_proposals WHERE owner_user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1','d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8') ORDER BY id" \
  > /tmp/opencode/r07-production-email-repair/private/proposals.tsv
chmod 600 /tmp/opencode/r07-production-email-repair/private/*.tsv
```

Expected: no terminal output. The private directory contains five nonempty-or-valid-empty TSV files. Abort if any export command fails. The helper in Task 3 is the dry run: it must exit zero and emit only counts after it proves the full transfer/delete topology. Any ambiguity, unapproved canonical collision, mapping mismatch, extra credential reference, or unexpected session decode aborts before a mutating script exists.

### Task 3: Produce Rust-Canonical Values And A Fully Bound Private Transaction

**Files:**
- Create: `/tmp/opencode/r07-production-email-repair/r07-repair-helper/Cargo.toml`
- Create: `/tmp/opencode/r07-production-email-repair/r07-repair-helper/src/main.rs`
- Create: `/tmp/opencode/r07-production-email-repair/repair.sql`
- Create: `/tmp/opencode/r07-production-email-repair/private/preflight-manifest.json`
- Test: `cargo run --manifest-path` for the one disposable crate

**Interfaces:**
- Consumes: Task 2 private TSVs and the approved UUID mapping.
- Produces: a private SQL file containing every exact captured ID/count and safely quoted psql variable for every old/canonical email value; it never writes values to stdout.

- [ ] **Step 1: Create the smallest disposable standalone helper**

Create `/tmp/opencode/r07-production-email-repair/r07-repair-helper/Cargo.toml`:

```toml
[package]
name = "r07-repair-helper"
version = "0.1.0"
edition = "2021"

[dependencies]
rmp-serde = "1.3.0"
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
uuid = { version = "1.16.0", features = ["serde"] }
```

Create `src/main.rs` as a single binary with this fixed contract:

```text
Inputs:
  --emails private/emails.tsv
  --sessions private/sessions.tsv
  --game-players private/game-players.tsv
  --proposal-players private/proposal-players.tsv
  --proposals private/proposals.tsv
  --sql repair.sql
  --manifest private/preflight-manifest.json

Output:
  stdout is exactly one JSON object containing only integer counts and the fixed
  string "dry_run_passed". stderr contains only generic failures and no TSV field.

Email algorithm:
  canonical(input) { input.trim().to_lowercase() }
  Parse every `emails.tsv` row as UUID, UUID, text. Build canonical collision
  groups. Permit exactly two groups of two: the approved retained/deleted row
  pairs. Reject a collision containing any other row (a real duplicate outside
  the approved mapping is still an abort, not a skip). R-07 canonicalizes only
  the two approved retained-survivor rows (`cfa2cca4-54d0-4d2d-b93c-e3a036e41f74`,
  `76ae8efa-2cd1-4cc4-9fa4-444c1ca723da`) into the canonicalization list. Every
  other noncanonical row - including a singleton with no collision - is left
  untouched by this repair: it is not an error and not included in the
  canonicalization list, only excluded. Migration 026's own blanket
  `UPDATE ... WHERE email <> lower(btrim(email))` canonicalizes any such row
  separately when 026 later runs; R-07 does not need to and must not touch it.
  Specifically excluded by this rule: `user_emails.id = '0ad09a53-ea45-495e-ae46-820245f2bcbb'`
  (owner `0b9208e6-6bd9-435b-b352-8d4dc3cff3e4`) - its stored value is not
  case/whitespace noise but free text containing an embedded, unrelated
  address; it is a singleton (no collision) and out of the approved mapping,
  so R-07 leaves it exactly as stored.

Session algorithm:
  Decode every hex `data` field with `rmp_serde::from_slice` into the same
  three-field tower_sessions Record layout: id, data map, expiry_date. Read only
  data["user"].id as UUID. Select IDs only when it equals either loser UUID.
  Decode failure, malformed user value, or an unexpected TSV shape fails closed.
  Do not print an ID, payload, name, or email.

Topology algorithm:
  Verify every captured game-player, proposal-player, and proposal-owner row has
  an approved loser or survivor owner. Verify no loser/survivor pair shares a
  game, no loser proposal-player projects onto an existing survivor proposal
  player, and no loser-owned proposal has the survivor as its player. Capture the
  complete immutable ID set and count for each transferred collection.

SQL algorithm:
  Emit `\\set` variables for every old and Rust-canonical email, using a psql
  literal encoder that rejects NUL and escapes quote and backslash characters.
  Emit UUID literals only after `Uuid::parse_str` succeeds. Emit the decoded
  session IDs and all captured game/proposal IDs into transaction-local VALUES
  tables. Emit assertions for every count and exact ID set before and after each
  mutation. Do not emit SQL for an unapproved row.
```

The helper must be outside the workspace. It must not import `web`, invoke Cargo for the workspace, access Kubernetes, or open a network connection. Its canonical function must be the literal Rust expression `input.trim().to_lowercase()`.

- [ ] **Step 2: Build and run the private dry run without using any web Cargo command**

```bash
cargo run --quiet --manifest-path /tmp/opencode/r07-production-email-repair/r07-repair-helper/Cargo.toml -- \
  --emails /tmp/opencode/r07-production-email-repair/private/emails.tsv \
  --sessions /tmp/opencode/r07-production-email-repair/private/sessions.tsv \
  --game-players /tmp/opencode/r07-production-email-repair/private/game-players.tsv \
  --proposal-players /tmp/opencode/r07-production-email-repair/private/proposal-players.tsv \
  --proposals /tmp/opencode/r07-production-email-repair/private/proposals.tsv \
  --sql /tmp/opencode/r07-production-email-repair/repair.sql \
  --manifest /tmp/opencode/r07-production-email-repair/private/preflight-manifest.json
chmod 600 /tmp/opencode/r07-production-email-repair/repair.sql
```

Expected safe output: a single JSON count object with `"status":"dry_run_passed"`, two approved collision groups, and no email/name/session field. Abort on any helper error, a collision involving an unaffected row, an unapproved noncanonical row, a mapping mismatch, a topology ambiguity, an unreadable session, or a private output file that is missing.

- [ ] **Step 3: Inspect only the generated SQL structure and counts**

```bash
grep -E '^(BEGIN;|SET TRANSACTION|CREATE TEMP TABLE|DELETE FROM|UPDATE |COMMIT;|ROLLBACK;)' /tmp/opencode/r07-production-email-repair/repair.sql
jq '{status, approved_collision_groups, retained_rows_to_canonicalize, singleton_rows_to_canonicalize, loser_sessions, transferred_game_players, transferred_proposal_players, transferred_proposals}' /tmp/opencode/r07-production-email-repair/private/preflight-manifest.json
```

Expected: the SQL begins a single transaction, contains no `CREATE`, `ALTER`, `DROP`, or migration statement outside `CREATE TEMP TABLE`, and ends with `COMMIT;`. The JSON has count fields only. Abort if an inspected SQL line names `games`, `game_logs`, `game_log_targets`, `chats`, `login_email_sends`, or any unapproved target table.

### Task 4: Execute The Single Serializable Repair Transaction

**Files:**
- Create: `/tmp/opencode/r07-production-email-repair/repair.sql` from Task 3
- Modify: production data only within the one transaction
- Test: SQL assertions inside the transaction

**Interfaces:**
- Consumes: completed Backup, passing Task 2 preflight, and Task 3's private fully bound SQL.
- Produces: two deleted loser users, transferred game/proposal references, the two canonical retained-survivor email rows, and no partial mutation on error.

- [ ] **Step 1: Confirm the generated transaction has the required fixed sequence**

The generated `repair.sql` must contain this exact operation order and assert each captured affected-row count/ID set with `RAISE EXCEPTION` before `COMMIT`:

```sql
BEGIN;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '60s';
SET LOCAL idle_in_transaction_session_timeout = '60s';
```

**Precondition (execution ordering, load-bearing):** this transaction runs against production at its current schema state, migration 022, before any further migration is applied. The full pending batch (023-032) runs afterward, uninterrupted, in one migration window. This is why Step 2 below does not touch `settings_email_token`/`settings_token_expires_at`/`settings_token_used_at`/`unsubscribe_token`: those columns do not exist yet at 022 (they originate in migrations 023, 027, and 025 respectively) and the repair must not assume any schema state beyond what production actually has when it runs.

1. Lock approved users and email rows with `FOR UPDATE`; assert four users, four approved rows, exact row-owner pairs, exactly one email row per approved user, and every captured old email.
2. Delete loser `public.user_auth_tokens`, matching `public.login_confirmations`, and decoded `tower_sessions.session` IDs. Do not attempt to clear settings/unsubscribe token columns first - they do not exist at migration 022 (they are added by migrations 023/025/027, none applied at repair time) and are moot regardless, since the loser `users` row that would hold them is deleted outright in Step 7.
3. Update `public.game_players.user_id` from each loser to its survivor and set `email_token = NULL`; assert the exact captured player ID sets, game IDs, and counts.
4. Update `public.game_proposal_players.user_id` and clear `email_token`; update `public.game_proposals.owner_user_id`; assert exact captured proposal-player IDs, proposal IDs, owner IDs, and counts.
5. Delete only approved ancillary rows in FK-safe order: `public.chat_messages` through loser `public.chat_users`, loser `public.chat_users`, loser endpoints in `public.friends`, loser endpoints in `public.blocks`, and loser `public.game_type_users`.
6. Delete exactly the two approved loser `public.user_emails` IDs. Update only the two retained survivor rows with their bound Rust-canonical strings. Assert all exact old/new values and exact affected IDs.
7. Delete exactly the two loser `public.users` IDs. Assert zero references in each of the 11 direct-FK locations, zero decoded loser sessions, zero matching login confirmations, and the preserved game/proposal identities/counts.

The final statements are exactly:

```sql
COMMIT;
```

The script must be noninteractive and must not use `lower(btrim(email))` to assign an email value. Abort if any required assertion is absent or the generated script includes a mutation not listed above.

- [ ] **Step 2: Execute once and preserve rollback behavior**

```bash
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec -i --namespace=brdgme -c postgres "$PGPOD" -- psql --no-psqlrc -X -q -v ON_ERROR_STOP=1 -U postgres -d brdgme < /tmp/opencode/r07-production-email-repair/repair.sql
```

Expected safe output: psql command tags and a successful `COMMIT`, with no email, name, token, or session payload. A lock timeout, statement timeout, idle timeout, serialization failure, assertion failure, or nonzero psql exit aborts the run. Before `COMMIT`, PostgreSQL rolls back the entire transaction; do not retry. After `COMMIT`, do not make compensating writes: stop and use the verified CNPG recovery point only if the owner directs recovery.

### Task 5: Verify The Completed Repair Without Applying Migrations 026 Or 029

**Files:**
- Modify: none
- Test: count-only read-only postcheck plus private Rust helper rerun

**Interfaces:**
- Consumes: Task 4 committed transaction and Task 3 private helper/manifest.
- Produces: production evidence that the repair is complete and 026 and 029 are ready but unapplied.

- [ ] **Step 1: Run the read-only zero-reference and preservation checks**

Run a `BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY` psql script with the Task 2 local timeouts and final `ROLLBACK`. It must emit only these count comparisons:

```sql
SELECT 'loser_users=' || count(*) FROM public.users
WHERE id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8');

SELECT 'remaining_direct_fk_references=' || sum(reference_count) FROM (
  SELECT count(*) AS reference_count FROM public.user_emails WHERE user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.user_auth_tokens WHERE user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.friends WHERE source_user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8') OR target_user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.blocks WHERE blocker_user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8') OR blocked_user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.chat_users WHERE user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.game_type_users WHERE user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.game_players WHERE user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.game_proposals WHERE owner_user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
  UNION ALL SELECT count(*) FROM public.game_proposal_players WHERE user_id IN ('faf09a2d-09c1-4f22-a1a2-88e8eb95cdd1', 'd5197dc5-cfa3-48f3-a5d5-f2aef8ebace8')
) ref_counts;

SELECT 'games_total=' || count(*) FROM public.games;
SELECT 'proposals_total=' || count(*) FROM public.game_proposals;
SELECT 'migration_026_rows=' || count(*) FROM public._sqlx_migrations WHERE version = 26;
SELECT 'migration_029_rows=' || count(*) FROM public._sqlx_migrations WHERE version = 29;
SELECT 'sql_noncanonical=' || count(*) FROM public.user_emails WHERE email <> lower(btrim(email));
SELECT 'sql_duplicate_groups=' || count(*) FROM (SELECT lower(btrim(email)) FROM public.user_emails GROUP BY 1 HAVING count(*) > 1) duplicate_groups;
```

Expected: `loser_users=0`, `remaining_direct_fk_references=0`, `migration_026_rows=0`, `migration_029_rows=0`, and `sql_duplicate_groups=0`. `sql_noncanonical=1` is expected, not 0: R-07 deliberately excludes `user_emails.id = '0ad09a53-ea45-495e-ae46-820245f2bcbb'` (see Task 3's email algorithm) from canonicalization, and that row remains noncanonical until migration 026's own blanket update runs. `games_total` and `proposals_total` must equal Task 2's recorded counts. Abort tracker completion on any other mismatch. Do not apply migration 026 or 029.

- [ ] **Step 2: Re-export private values and rerun Rust-level checks**

Repeat Task 2's five exports into new `post-` files in the private directory, then rerun the helper with those paths and a distinct output SQL path. Expected safe output: `"status":"dry_run_passed"`, zero loser sessions, zero Rust-canonical duplicate groups, exactly one Rust-noncanonical row (the excluded singleton above, unchanged by design), and preserved game/player/proposal identity/count sets. Abort if any count differs from the Task 2 manifest other than the explicitly deleted users/emails/ancillary rows and transferred owner/user IDs.

- [ ] **Step 3: Record the explicit migration-026/029 decision**

Record this count-only conclusion in the operator's production evidence: `migration_026_rows=0`, `migration_029_rows=0`, the Rust helper reports zero duplicate rows and exactly the one known excluded noncanonical row, and the SQL diagnostics report `sql_noncanonical=1` (the same excluded row) and `sql_duplicate_groups=0`. The decision is: migrations 026 and 029 are ready and remain unapplied. Do not invoke `sqlx`, the migration Job, ArgoCD sync, or any migration command.

### Task 6: Update The R-07 Tracker And Commit The Named File Only

**Files:**
- Modify: `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md`
- Test: `git diff --check` and exact staged-file inspection

**Interfaces:**
- Consumes: all Task 5 postchecks passed and the exact implementation commit SHA resolved from git.
- Produces: one committed tracker update recording R-07 as done; then Orchestrator proceeds to R-08.

- [ ] **Step 1: Resolve exact commit identifiers immediately before editing**

```bash
git rev-parse 1e19d05f0506aa6e92cc16764d4f8c2f148eb022
git status --short
```

Expected: the implementation SHA is 40 characters and status may include the known untracked `docs/reviews/2026-07-30-review-session/R-07-HANDOVER.md`. Do not stage, delete, or alter that handover or any unrelated file. The production repair is an operational action, not a source change: record its date, verified Backup name, and count-only evidence without inventing a data-repair commit SHA.

- [ ] **Step 2: Mark R-07 complete with implementation/data-repair state and production evidence**

Update only the R-07 work-package entry in `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md` to state all of the following factual evidence:

```text
done
implementation: 1e19d05f0506aa6e92cc16764d4f8c2f148eb022
production repair: executed <actual UTC execution date>; CNPG Backup <actual generated BACKUP_NAME from Task 1 Step 2> completed and recovery point advanced (confirmed via ObjectStore.status.serverRecoveryWindow, not the deprecated Cluster.status field)
data result: 2 loser users and their approved email rows deleted; the 2 retained survivor email rows Rust-canonicalized; game/proposal participation and ownership transferred
verification: all postchecks passed; migrations 026 and 029 remain unapplied and ready
```

Substitute the placeholders with the real execution date and the real generated Backup name from `/tmp/opencode/r07-production-email-repair/backup-name.txt` (Task 1 Step 2) - never the literal `postgres-pre-repair-r07-20260801-01` string, which was this plan's original authoring-date placeholder, not a fixed identifier.

Step 1 must return `1e19d05f0506aa6e92cc16764d4f8c2f148eb022`; abort if it does not. Do not write secrets, PII, email values, tokens, connection strings, or a fabricated repair commit SHA.

- [ ] **Step 3: Commit the tracker as the exact named file**

```bash
git diff --check -- docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md
git add docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md
git diff --cached --name-only
git commit -m "docs(review): mark R-07 complete after production repair"
git rev-parse HEAD
```

Expected staged-file output contains only `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md`. Record the returned 40-character tracker commit SHA in the private evidence. Never push. After this commit, Orchestrator proceeds to R-08.

## Inline Self-Review

- [x] Coverage: Tasks 1-6 cover the named CNPG plugin Backup, bounded recovery verification, locked read-only/dry-run preflight, Rust-only canonical source, one serializable transaction, postchecks, migration-026/029 non-application, tracker evidence, exact-file staging, no push, and R-08 handoff.
- [x] Placeholder scan: removed open-ended task text and omitted generic future-work markers. Dynamic production values are supplied only by the precisely defined private helper from locked preflight snapshots, never guessed or manually interpolated.
- [x] Schema and command review: uses `public._sqlx_migrations`, `tower_sessions.session`, the exact 11 direct user FK locations, `barman-cloud.cloudnative-pg.io`, a freshly-generated timestamped Backup name (never hardcoded - see Task 1 Step 2), bounded ten-attempt polling, and the approved UUID mappings.
- [x] Identifier review: corrected every group-2 UUID occurrence to `d5197dc5-cfa3-48f3-a5d5-f2aef8ebace8`; retained/deleted email IDs and survivor/loser IDs match the approved mapping table.
