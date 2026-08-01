# Unit 10b - dependency & workspace hygiene

Continuation of Unit 10. 10a covered bot/operator/tools (F-186..F-196). This unit covers the
10 dependency/workspace-hygiene commits: WP-64 `4fb252da`, WP-65 `2c28ae85`, WP-66 `667c8f42`
(call sites), WP-67 `634c72db`, WP-69 `e2ee5342`+`be185ccb`, WP-70 `8304baf5`, WP-72 `a5d6f102`,
WP-73 `22d00b8d`, `22b68689`.

Findings continue from F-197. **Unit 10b was killed by a quota limit after F-202; Unit 10c
(2026-07-31) resumed in this same file and completed the remaining scope - the WP-66 wrap-up plus
WP-64, WP-67, WP-69 x2, WP-70, WP-72 and `22b68689`. 10c's findings run from F-203.** Nothing
from 10b was redone.

## Progress

- [x] Baseline established (Worker 1). **The breakdown's premise is wrong a THIRD time:**

| SHA | WP | git stat | **real files** (excl. `.sqlx/`, `Cargo.lock`) |
|---|---|---|---|
| `4fb252da` | WP-64 | 43 | 42 |
| `2c28ae85` | WP-65 | 35 | 35 |
| `667c8f42` | WP-66 | 101 | **12** (88 are `.sqlx/`, 1 lockfile) |
| `634c72db` | WP-67 | 1 | 1 |
| `e2ee5342` | WP-69 | 1 | 1 |
| `be185ccb` | WP-69 docs | 1 | 1 (docs only) |
| `8304baf5` | WP-70 | 6 | 5 |
| `a5d6f102` | WP-72 | 1 | 1 (comment only) |
| `22d00b8d` | WP-73 | 140 | 139 (135 `rust/game`, 2 `rust/lib`, 1 root manifest, 1 doc) |
| `22b68689` | - | 1 | 1 |

  **WP-66 is NOT a huge mechanical commit** - it is 12 real files and can be read in full.
  WP-73 genuinely is 139 files, but 135 are 3-line wrappers.

- [x] Spec existence verified directly (Worker 1). Present: WP-64, WP-66, WP-67, WP-69, WP-70, WP-73.
      **No spec: WP-65, WP-72.**
- [x] Checklist ownership verified per row (Worker 1):
  - **Only WP-65 owns checklist rows** - nine rows in `T3-B8-workspace-hygiene-red7-docs.md`,
    **every one `Test? = n`**. So there is **no "Test? y with no test" falsification available in this
    unit** - the unified report must not count any 10b row toward that tally.
  - WP-64/66/67/69/70/73 appear in the checklists only as cross-references or *explicit exclusions*
    ("BLOCKED-ON-DECISION D-19/D-20/D-23 ... not in this batch"). Untested by design, like WP-76/77.
  - **WP-72 appears in no checklist and has no spec.** It exists only as commit `a5d6f102`.
- [x] **WP-73 `22d00b8d` - CLEAN. Read exhaustively, not sampled.** See "Verified good".
      The two behaviour changes it does contain are both benign; the deployment side was already
      compatible. No pattern-2, no hidden divergence.
- [x] **WP-65 `2c28ae85` - all nine rows checked individually against the end state.**
      5 PASS, 1 PARTIAL FAIL (`e F28`, F-197), 1 PASS-but-weak (`dp F23`, F-199),
      3 dispositioned rather than implemented (`dp F21`/`dp F22`/`dp F9`) - dispositions verified.
- [x] rustls process-default sweep across all three binaries - F-198
- [x] **WP-66 `667c8f42` - 12-file call-site read COMPLETE** (Worker report `w4-wp66.md`, survived
      the quota kill). All 18 spec criteria scored; F-200/F-201/F-202 filed. Wrap-up done in 10c.
- [x] WP-64/67/69/70/72 + `22b68689` - **all completed by Unit 10c below.**

### Unit 10c (continuation after quota kill)

- [x] WP-66 wrap-up - both open items dispositioned: `deny.toml [licenses.private]` benign,
      `default-features = false` narrowing **REFUTED** (both below). No new WP-66 finding.
- [x] **WP-64 `4fb252da` - checked against all of spec 3a-3c and riders 1-6.** Largely clean;
      the four headline hunts all come back **negative** (see "Verified good"). Two Low findings:
      F-203, F-204.
- [x] **WP-67 `634c72db`** - the `sentry` trim is harmless but closed a finding whose stated
      mechanism was **false**, and the spec's rider 2 downgrade never happened. **F-205**, a new
      named pattern.
- [x] **WP-69 `e2ee5342` + `be185ccb`** - **F-206 (Medium), this unit's most serious finding**:
      the spec's explicit STOP-AND-REPORT threshold fired (29 skip entries vs "roughly a dozen")
      and the commit wrote a pre-emptive rebuttal into `deny.toml` instead of stopping. All 29
      entries checked individually. `be185ccb` is a harmless bookkeeping error, not pattern 4b.
- [x] **WP-70 `8304baf5`** - `serde_yaml_ng` fork risk **REFUTED by full `diff -ru`** of both
      crate source trees; all 7 call sites at HEAD were in the diff. No finding.
- [x] WP-72 `a5d6f102` - comment content confirmed accurate, no finding. See below.
- [x] **`22b68689`** - adds `cargo-deny` to `devenv.nix:31`. Correct, and it is the remediation
      for the `docs/CODING.md:547-553` no-global-install directive. No finding.
- [x] Two Lead-originated questions answered: sqlx-cli version split (**F-207**) and the
      `deny.toml` self-contradiction (folded into **F-206**).

**UNIT 10 IS NOW FULLY CLOSED.** 10a + 10b + 10c between them cover all 10 dependency/workspace
commits plus bot/operator/tools. Findings F-186..F-207.

## Findings

### F-197 (Low) - pattern 2 in WP-65's `e F28` sweep: four byte-identical siblings left behind

`e F28`'s row named three crates and the commit cleaned exactly those three, literally satisfying
the row. The stale files it was for survive elsewhere, and the same commit *touched two of the
missed crates for a different reason*:

| Survivor | Content | Why it is a sibling |
|---|---|---|
| `rust/game/love-letter-2/.rls.toml` | `build_lib = true` | **byte-identical** to the `.rls.toml` deleted from `acquire-1` and `lost-cities-2` in `2c28ae85` |
| `rust/lib/rand_bot/.rls.toml` | same | same artefact, outside `rust/game/` so outside the row's file list entirely |
| `rust/game/love-letter-2/.gitignore` | still has `lambda`, `.vscode`, `.idea` | the exact three lines stripped from `lost-cities-2/.gitignore` in the same commit |
| `rust/game/modern-art-2/.gitignore` | **byte-identical to the above** | same |

- **The miss is demonstrable, not inferred**: `2c28ae85` edits `rust/game/love-letter-2/src/lib.rs`
  and `rust/game/modern-art-2/src/lib.rs` (the `e F9` `mod test` -> `mod tests` rename) while
  leaving their `.gitignore`/`.rls.toml` alone, and in the *same commit* removes those exact
  lines from `lost-cities-2/.gitignore`. The three `.gitignore` files were byte-identical before
  the commit.
- **Why it matters**: cosmetic in isolation, but it is a clean textbook instance of the session's
  most productive pattern in a work package whose entire purpose was a *sweep*. A sweep that
  works from an enumerated file list rather than a content predicate cannot be complete, and this
  one was accepted as complete.
- **What did land**: `build-release` is fully eradicated (zero survivors workspace-wide), and the
  three named crates are clean.
- **Fix**: `rg --files -g '.rls.toml' rust/` and delete all; strip `lambda`/`.vscode`/`.idea` from
  every `rust/**/.gitignore`. One line each.

### F-198 (Low) - `rust/bot` is the only TLS-capable binary with no rustls process-default install

`rust/web/src/main.rs:14-23` and `rust/operator/src/main.rs:21-24` both open with

```rust
rustls::crypto::aws_lc_rs::default_provider()
    .install_default()
    .expect("failed to install rustls crypto provider");
```

These are the **only two** `install_default`/`CryptoProvider` sites in the workspace.
`rust/bot/src/main.rs:776-827` has none, and `rust/bot/Cargo.toml` declares no `rustls`
dependency at all, so the call is not even expressible without a manifest change - despite the
bot's graph carrying `reqwest` (`rustls`), `sqlx` (workspace `tls-rustls`) and `async-nats`.

**Stated honestly: I cannot demonstrate a live panic, and the finding is filed at Low for that
reason.** The evidence and its limits:

- `docs/CODING.md:579-598`'s rule is **conditional**, not universal: *"Any binary using a crate
  that reads the process default provider (today: `kube` in the operator) must call
  `rustls::crypto::aws_lc_rs::default_provider().install_default()` at the top of `main` ...
  when in doubt, install the default - it is always safe."* The bot has no `kube` dependency, so
  **by the letter of the rule the bot is compliant.** This is not a checklist falsification.
- The doc's immunity analysis covers `sqlx`, `reqwest` and `kube` - and **omits `async-nats`**,
  which the same doc lists (`:582-583`) among the crates pulling `ring` in. That omission is
  exactly where the bot sits. `async-nats`' provider-selection behaviour was not read, so the
  gap is unresolved rather than shown to be a defect.
- **Why it is still worth filing**: the condition is invisible today only because in-cluster NATS
  and Postgres are plaintext (`k8s/base/bot/deployment.yaml:33` sets `NATS_URL` with no TLS or
  `sslmode` anywhere under `k8s/base/bot/`). CODING.md itself records this class first surfacing
  as *"the operator CrashLooping in prod (2026-07-08)"* - i.e. the failure mode is a startup
  crash loop, discovered in production, and the mitigating condition is a deployment detail that
  a TLS rollout would remove.
- **Not a WP-64 regression - checked.** `4fb252da` only lifted the already-existing per-crate
  `rustls = { version = "0.23", default-features = false, features = ["aws-lc-rs"] }` in web and
  operator into `[workspace.dependencies]`; both explanatory comments appear as *context* lines
  in that diff, not additions. `git log -S rustls -- rust/bot/Cargo.toml` returns only the bot's
  original creation and WP-66, both touching feature strings. **The bot has never had a direct
  `rustls` dep.** The omission is original, WP-64 inherited and centralised it.
- **Fix**: add `rustls.workspace = true` to `rust/bot/Cargo.toml` plus the same three lines at the
  top of `main`. Three lines, endorsed verbatim by CODING.md, and it makes the three binaries
  uniform. Independently, extend CODING.md's immunity list to cover `async-nats` or state that it
  is unverified.

### F-199 (Low) - `deps-currency.yml` satisfies `dp F23` mechanically but cannot alert anyone

`.github/workflows/deps-currency.yml` (created by `2c28ae85` itself; no other commit touches it):
`cron: '0 6 * * 1'` + `workflow_dispatch`, one job running `cargo deny check advisories` in
`rust/`.

- It **can** fail (non-zero exit on an advisory hit), so the row is honestly satisfied.
- But there is **no notification wiring** - no issue-creation step, no webhook, no annotation. A
  scheduled workflow failing on the default branch relies entirely on GitHub's per-user "Actions
  failure" email, which is the single most commonly muted GitHub notification. The row's own
  words were "so version drift is caught **mechanically** instead of by review"; a red run nobody
  is told about is still caught by review.
- It also checks **`advisories` only**. `deny.toml`'s `bans`, `licenses` and `sources` sections -
  the parts WP-69 hardened - are never exercised on a schedule. They only run on the PR job
  (`.github/workflows/ci.yml:96-109`, `cargo deny check`), which is gated on
  `needs.changes.outputs.rust == 'true'`. **A newly-yanked crate, a licence change in a
  transitive dep, or a new duplicate introduced by a `cargo update` in a non-`rust/**` PR is
  therefore invisible until the next `rust/**` change.**
- **Fix**: add an issue-creating step (or make the weekly job run the full `cargo deny check`),
  and drop the `advisories`-only narrowing.
- Note for the tally: `dp F23`'s checklist row is **`Test? = n`**, so this is not a falsified row.

### F-200 (Medium) - the vendored session store's `migrate()` can report success without creating the session table, and it is the *only* thing that creates it

`rust/lib/session_store/src/postgres_store.rs:87-130`:

```rust
    pub async fn migrate(&self) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;
        ...
        if let Err(err) = sqlx::query(AssertSqlSafe(create_schema_query)).execute(&mut *tx).await {
            if !err.to_string().contains("duplicate key value violates unique constraint") {
                return Err(err);
            }
            return Ok(());          // <-- returns BEFORE `create table`, and without committing
        }
        // create table if not exists ...
        tx.commit().await?;
```

On the duplicate-key path the function returns `Ok(())` **before** running `create table if not
exists` and **without committing** - `tx` is dropped, so the transaction rolls back.

- **Concrete failing path**: a first-ever deploy against an empty database with more than one
  `web` replica. Both call `migrate()` (`rust/web/src/auth/session.rs:38-42`, on **every** process
  start, with `.expect("Failed to run session store migration")`). One wins `create schema`; the
  loser hits the duplicate-key branch, returns `Ok(())`, and **never creates
  `tower_sessions.session`**. Startup reports success. Every subsequent session write then fails.
  If the winner is also the pod that is rescheduled first, no restart repairs it.
- **Why it matters here specifically**: `rg 'tower_sessions' rust/web/migrations/` returns **zero
  hits**. The table is not created by any sqlx migration - `migrate()` is its **sole** creator.
  The WP-66 spec's crit 8 mandated exactly this arrangement ("via its own `migrate()` - not via
  `rust/web/migrations/`"), and its crit 13 asserts "`migrate()` on an already-migrated database
  is a no-op". The already-migrated case is fine; the *cold* case is not, and nothing tests it.
- **NOT introduced by the vendoring - this is a faithful copy.** Verified by direct diff against
  `tower-sessions-sqlx-store-0.15.0/src/postgres_store.rs:86-107` in the local registry: the block
  is verbatim upstream, differing only by the `AssertSqlSafe` wrapper. **That is precisely why it
  is worth filing**: WP-66 converted a third-party bug into first-party code that brdgme now owns
  and can fix, and the "minimal port, not a rewrite" criterion (correctly followed) guaranteed it
  came along. A vendoring work package's spec should carry a "known upstream defects inherited"
  criterion; this one had none.
- **Second-order risk in the same block**: the classification is a **substring match on an sqlx
  error's `Display` output**. WP-66 carried it across a **sqlx 0.8 -> 0.9 major bump** without
  re-checking the message text. If 0.9 reformatted it, `migrate()` now propagates the concurrent-
  create race as a hard `.expect()` panic at startup instead of swallowing it. Unverified from the
  repo, but it is the same class as F-201.
- **Fix**: replace the duplicate-key branch with `continue to create table` (drop the early
  `return Ok(())`), or make the whole thing `create schema if not exists` + `create table if not
  exists` in one committed transaction and classify on SQLSTATE `23505`/`42P06` rather than on
  message text. Add a test that runs `migrate()` twice concurrently against a fresh database.

### F-201 (Low) - three sqlx error-classification sites crossed the 0.8 -> 0.9 major bump unexamined

`667c8f42` touched exactly four `.rs` files: three `use`-path swaps and one `AssertSqlSafe` wrap.
It did **not** touch, and did not add a test for, any of the sites whose behaviour depends on the
`DatabaseError` trait it just bumped a major version of:

| Site | Code |
|---|---|
| `rust/web/src/db/users.rs:256` | `Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(false)` |
| `rust/web/src/db/emails.rs:101` | `Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(None)` |
| `rust/web/src/game/import.rs:199` | `Err(sqlx::Error::Database(de)) if de.is_unique_violation() =>` |

- **Stated honestly: I cannot demonstrate that sqlx 0.9 changed `code()` or
  `is_unique_violation()` semantics for Postgres, so this is filed at Low as an unverified risk,
  not a defect.** The point is procedural: these three are the *only* places in the workspace
  where a sqlx error is classified rather than propagated, they are the highest-risk surface of a
  major bump, and the WP-66 spec's test section (crit 13) lists session/cookie behaviour and says
  nothing about them.
- **Why it matters**: all three convert a unique-violation into a *non-error* control-flow branch
  (`Ok(false)`, `Ok(None)`, a fallback). If classification silently stopped matching, the failure
  is not a crash - it is a duplicate-email or duplicate-user error surfacing as a 500 instead of a
  clean "already taken", and on `import.rs:199` an import aborting instead of skipping. Silent
  degradation, exactly the shape a mechanical diff hides.
- **Fix**: one test per site asserting the classification still fires, pinned in the same change
  as any future sqlx bump.

### F-202 (Low) - the sqlx 0.9 migration mechanically silenced the one warning sqlx 0.9 added

`rust/web/src/db/test_support.rs:146-152`:

```rust
pub(crate) async fn count_rows(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM {}",
        table
    )))
```

`table: &str` is interpolated with **no validation whatsoever**. sqlx 0.9 introduced
`AssertSqlSafe` specifically to make an unchecked dynamic-SQL construction *say so at the call
site*; the migration satisfied the compiler by adding the wrapper and audited nothing.

- **Not a live injection surface, and not introduced here** - the pre-image was `&format!(...)`,
  equally unvalidated. All callers pass string literals (`rust/web/src/db/social.rs:621,874,882`;
  `rust/web/src/db/game_write.rs:1629-1641`), and the function is `pub(crate)` test-support.
- **Why it is still worth a line**: the safety property is enforced by a doc comment ("never pass
  a runtime value for `table` (ws F51(3))") and caller discipline only. Contrast the vendored
  store's eight `AssertSqlSafe` sites, which are genuinely safe: their interpolated values are the
  constants `"tower_sessions"`/`"session"` set in `new()`, changeable only through
  `with_schema_name`/`with_table_name`, both of which reject anything failing `is_valid_identifier`
  (`rust/lib/session_store/src/postgres_store.rs:257-267`) - **and neither builder has a caller in
  `rust/web`**. The store did the work; the one first-party site did not.
- **Fix**: take `table: &'static str`, or run the same `is_valid_identifier` check. One line.

### F-203 (Low) - WP-64 shipped `[workspace.lints.clippy]` but silently dropped the prescribed `[workspace.lints.rust]` table

WP-64's spec §3c prescribes **two** tables, `[workspace.lints.rust]` **and**
`[workspace.lints.clippy]`. `rust/Cargo.toml:78-79` at HEAD has only:

```toml
[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
```

There is no `[workspace.lints.rust]` table, and none was ever added by a later commit.

- **Why it is a real gap rather than a nit**: `[lints] workspace = true` is present on all 44
  members (proved by 10b), so the *mechanism* is fully wired and the opt-in cost has already been
  paid on every crate. The rustc-lint half of it simply carries nothing. Adding
  `unsafe_code = "forbid"`, `rust_2018_idioms`, `unused_qualifications` or similar is now a
  two-line change with workspace-wide reach - the expensive part is already done.
- **It is an omission, not a documented deviation.** WP-64's rider 4 permits *dropping an
  individual lint that fires*; it does not authorise omitting the whole table. The commit message
  is a bare subject line and says nothing about it, so nothing records the decision either way.
- **Honest limit**: I cannot demonstrate a defect that the missing table would have caught, and
  Worker evidence shows there was **no** stricter per-crate config displaced
  (`git grep` for `#![deny/warn/allow]` at `4fb252da^` over `rust/` returns **zero hits**). So
  this is a missed opportunity plus a spec-vs-code discrepancy, filed at **Low**, not a
  regression.
- **Note for the tally**: WP-64 has no checklist row (deferred, BLOCKED-ON-DECISION), so this is
  **not** a falsified `Test? y` row.

### F-204 (Low) - WP-64's own rider 1 is violated by the table it created, and the spec contradicts itself

Rider 1: *"never leave a bare-major spelling"*. Ten of the 21 `[workspace.dependencies]` entries
in `rust/Cargo.toml:56-76` are bare-major or bare-minor: `tracing = "0.1"`, `time = "0.3"`,
`tracing-subscriber = "0.3"`, `hex = "0.4"`, `sentry = "0.48"`, `sentry-tracing = "0.48"`,
`reqwest = "0.13"`, `rustls = "0.23"`, `aes-gcm = "0.10"`, `sqlx = "0.9"` (the last set by WP-66,
not WP-64).

- **Filed low and with a caveat, because the spec argues against itself**: §3b also says to use
  "the most precise spelling in use", and for several of these the pre-image manifests carried the
  bare spelling too, so §3b endorses exactly what rider 1 forbids. **A reviewer cannot call this
  a clean falsification** - it is an internally inconsistent acceptance criterion, which is itself
  worth recording.
- **Practical consequence is small but non-zero**: it interacts with `docs/CODING.md:540-545`
  ("stay on latest dependencies") - a bare `"0.3"` on a 0.x crate accepts any 0.3.x, and
  `cargo update` will silently take new 0.3 patch releases. That is the intended policy here, so
  the rider was arguably the wrong rule, not the code.
- **Also unmet, trivially**: rider 6 required the serde-`derive` decision be stated in the PR
  description; `4fb252da`'s commit message is a bare subject line with no body.

### F-205 (Low, but a NEW NAMED PATTERN) - WP-67 closed a finding whose stated mechanism was false, and the spec's own rider required a downgrade that never happened

`dp F12` asserted that `sentry`'s default features were "dragging actix-web + ureq into every
build". WP-67 `634c72db` trimmed `sentry` to an explicit feature list, and the finding was closed.
**The premise was never true.**

- `sentry` 0.48.5's own manifest (`~/.cargo/registry/.../sentry-0.48.5/Cargo.toml`) declares
  `default = ["backtrace", "contexts", "debug-images", "panic", "transport", "release-health"]`
  and `transport = ["reqwest", "native-tls"]`. **`actix` and `ureq` are not default features**,
  and nothing in the workspace enables either. Neither crate was ever compiled - before or after
  the trim.
- **Proof it is still true at HEAD, not just an argument about defaults**: `actix-web` and `ureq`
  remain `[[package]]` entries in `rust/Cargo.lock` **after** later commits (`8304baf5`)
  regenerated the lockfile. Their presence in the lock is a resolution artefact, not a build
  input - exactly as it was before WP-67.
- **The spec anticipated this and the anticipation was ignored.** WP-67's rider 2 required that,
  if the mechanism turned out to be false, the *downgrade be written back into the finding*.
  Instead: `docs/reviews/2026-07-23-rust-review/SUMMARY.md:44-46` still states the
  actix-web/ureq claim as fact, `SUMMARY.md:139` records no downgrade, and the archived
  `findings/dependencies.md:103-108` (and `:157`) is unamended.
- **Why this is worth a named pattern rather than a nit** - *"the finding whose premise was false,
  closed by a change that could not have fixed it"*. The trim itself is harmless and arguably
  good hygiene. What is wrong is that the corpus now records a **verified-closed** finding whose
  justification is untrue, so any future reader (or any sign-off procedure keyed on "was it
  closed?") inherits a false belief about the dependency graph. This is distinct from pattern 4b
  (docs edited to agree with wrong code): here the *docs were never edited at all* despite an
  explicit criterion requiring it. **Recommend the unified report add a sign-off step: when a
  fix's stated mechanism is disproved during implementation, the finding must be amended, not
  merely closed.**
- **Related trivium, same commit, not filed separately**: `rust/web/Cargo.toml:93-94` still
  carries the comment "Default features (reqwest + native-tls transport)". Its siblings carry no
  such comment, so this is a single stale site, **not** a pattern-2 instance.

### F-206 (Medium) - WP-69's spec set an explicit STOP-AND-REPORT threshold, the threshold fired, and the commit wrote a pre-emptive rebuttal into `deny.toml` instead of stopping

This is Unit 10c's most serious finding.

WP-69's spec §3b says that if enabling `multiple-versions = "deny"` requires **more than roughly a
dozen** `skip` entries, the implementer must **STOP and report rather than paper over it**.

- **The threshold fired, by a factor of ~2.4.** `rust/deny.toml:77-138` contains **29** skip
  entries. (10b's report said 24; the correct count is 29 - corrected here.)
- **The commit did not stop.** `e2ee5342` shipped all 29 and wrote, immediately above them
  (`rust/deny.toml:71-76`): *"These are ecosystem semver splits no single sibling was scoped to
  fix - **not papered-over sibling work**."* That sentence is a pre-emptive rebuttal of the exact
  escalation the spec mandated, placed in the artefact the escalation was about.
- **The rebuttal is falsified by an entry in the very list it introduces.** `rust/deny.toml:131`
  reads `{ name = "tower-http", version = "0.7.0" }, # (a) via web (first-party, pins 0.7.0
  directly)`. That duplicate is caused by brdgme's own direct pin at `rust/web/Cargo.toml:44`,
  not by any ecosystem split. **All 29 entries were checked individually: this is the only one
  whose `(a)` clause names a first-party cause.** One borderline case - `gloo-timers 0.3.0`
  (`:127`) - skips *upstream* `backon`'s copy, but the split exists only because
  `rust/web/Cargo.toml:84` pins 0.4 directly; that is disclosed in the block comment, so it is not
  a face-value contradiction. Of the six skip-list crates that are also direct workspace deps
  (`getrandom`, `rand`, `rand_chacha`, `thiserror`, `gloo-timers`, `tower-http`), **only
  `tower-http` skips the first-party copy.**
- **Provenance confirmed, not inferred**: `git log -S 'papered-over sibling work' -- rust/deny.toml`
  and `git log -S 'tower-http' -- rust/deny.toml` each return exactly `e2ee5342`. The blanket
  claim and the entry contradicting it landed in the same commit.
- **Partially mitigated, and this matters for severity**: `EXECUTION-STATE.md` *does* disclose a
  "~30-entry" skip list to the reader. So the size was not hidden from the programme - only the
  spec's own stop-condition was silently overridden. Filed **Medium**, not High, for that reason.
- **Compounding**: WP-69's spec §5 negative checks - the "the flip must actually bite" tests -
  are recorded in `EXECUTION-STATE.md` as **parked, not run**. So nothing ever demonstrated that
  `multiple-versions = "deny"` rejects a genuinely new duplicate rather than being fully neutered
  by the skip list. Combined with F-199 (the weekly job runs `advisories` only, never `bans`) and
  Coverage gap 3 (no expiry, no `unused-skip`), **the `bans` section's practical enforcement is
  unmeasured from end to end.**
- **Honest limit**: I could not run `cargo deny check`, so I cannot say whether any skip entry is
  already stale. That is the point - neither can anyone else, and nothing runs it on a schedule.
- **Fix**: (1) remove the `tower-http 0.7.0` skip and align `rust/web/Cargo.toml:44` with the
  0.6.11 canonical version, or amend the block comment to stop claiming all entries are upstream;
  (2) run WP-69 §5's parked negative checks; (3) add `unused = "warn"` to `[bans]` and give each
  skip an expiry review date.

### F-207 (Low) - three different sqlx migrators write `_sqlx_migrations`, and no commit or spec ever justified the split

WP-66 moved the workspace to sqlx **0.9**. The tooling did not follow, and the divergence was never
discussed anywhere:

| Migrator | Version | Where |
|---|---|---|
| `sqlx-cli` in the production `migrate` image | **pinned 0.8.6** | `rust/Dockerfile:132`, run at `:139` |
| `sqlx-cli` in CI | **unpinned (latest)** | `.github/workflows/ci.yml:90-92`, run by `scripts/rust-ci-commands.sh:12,24` |
| the `sqlx` library in `#[sqlx::test]` | **0.9** | `rust/operator/Cargo.toml:39`, `rust/web/Cargo.toml:30` |

- **No commit in the whole 127-commit range `f0589894..HEAD` touches `rust/Dockerfile` at all.**
  The 0.8.6 pin was last set by `c23e359`, which predates the review. WP-66's spec never mentions
  `sqlx-cli`, `0.8.6` or the Dockerfile. `docs/CODING.md:571` ("The `sqlx-cli` pin in
  `rust/Dockerfile` remains `0.8.6`") is the **sole** record and gives no reason.
- **Nothing at runtime validates migration checksums.** `rg 'migrate!' rust --glob '*.rs'` is
  **empty** - there is no `sqlx::migrate!` anywhere. The one non-test `.migrate()` call
  (`rust/web/src/auth/session.rs:41`) is the vendored session store's own idempotent DDL
  (`rust/lib/session_store/src/postgres_store.rs:87`) and never touches `_sqlx_migrations`.
  **That is what keeps this at Low**: the failure mode, if any, would be confined to the migrate
  job itself, not a web startup crash.
- **Stated honestly: whether sqlx 0.9 changed the `_sqlx_migrations` table or checksum format is
  NOT determinable from this repo** - sqlx-cli is not vendored and I will not speculate. The
  finding is that **CI validates migrations under a different sqlx-cli major than production
  applies them under**, with no record of anyone having considered it.
- **Fix**: pin CI's `sqlx-cli` to the same version as the Dockerfile (so CI actually tests what
  ships), and record a reason for whichever version is chosen.

## Unit 10c - WP-66 wrap-up

The surviving Worker report (`w4-wp66.md`) was re-read in full. F-200, F-201 and F-202 already
came from it. Its remaining scorecard is confirmed: criteria 1, 2, 4/5/7/8/9/10/12/14 and 17/18
all Met, criterion 6 (`minimal port`) **Met exactly by direct diff against the upstream 0.15.0
registry copy**, criterion 16 (getrandom rider) discharged elsewhere in `4f5f6d4` (WP-61) and
correctly so. Nothing else in the report rises to a finding. Two items 10b had not dispositioned
are judged below.

### `default-features = false` on the workspace sqlx entry - REFUTED, do not re-derive

`667c8f42` added `default-features = false` to `rust/Cargo.toml:76`, so `bot` and `operator`
(non-dev) lost sqlx's defaults `macros`, `migrate`, `json` and `any` relative to their pre-image,
unmentioned in the commit message. This is exactly the shape 10c was told to hunt ("a
`Cargo.toml` feature dropped as a side effect"). **It is nevertheless inert, demonstrated not
assumed:**

- `rust/bot/src/` uses only the sqlx **function** forms - `sqlx::query(...)` at
  `config.rs:26,50,62` and `main.rs:91,136,327,525` - plus `PgPool`/`Row`. Grep for
  `FromRow|query_as!|query_scalar!|migrate!|::Any` over `rust/bot/src/` returns **zero hits**.
  Bot re-enables `json` explicitly anyway (`rust/bot/Cargo.toml:17`).
- `rust/operator/src/` **non-test** code (through `controller.rs:294`; `#[cfg(test)] mod tests`
  starts at `:295`) likewise uses only `sqlx::query` (`:177,218,241`) and `sqlx::query_scalar`
  (`:202`). All four `#[sqlx::test(migrations = ...)]` sites (`:343,399,475,522`) are inside the
  test module and are served by the dev-dep at `rust/operator/Cargo.toml:39`
  (`features = ["macros", "migrate"]`), which is precisely what spec crit 3 prescribed.
- Operator binds no JSON: every `.bind()` is `&str`/`&[i32]`/`f32`/`bool`/`i32`/`Uuid`; its
  `serde_json` use is kube CRD status only.
- **All four dropped features are compile-time-only.** A miss is an unresolved-path compile
  error, never silent runtime degradation - the opposite of the failure mode that makes feature
  drift dangerous. There is no path by which this narrows behaviour.

**One residual, offered as a suggestion rather than a finding**: the comment at
`rust/operator/Cargo.toml:37-38` explains the dev-dep but not the deliberate omission of
`macros`/`migrate` from the main dep. A future `query!` in non-test operator code would produce a
confusing "not found in `sqlx`" error.

### `[licenses.private] ignore = true` (`rust/deny.toml:45-49`) - NOT a finding

`cargo-deny`'s `licenses.private.ignore` skips licence *expression* evaluation for
`publish = false` workspace members. That is the correct and conventional setting: brdgme's 44
members carry no `license` field and never will. **Turning it off would not machine-check the
vendored MIT obligations either** - it would only require `brdgme_session_store` to declare
`license = "MIT"` in its manifest, which is a string, not a check that `LICENSE` and the
attribution header still exist. No `cargo-deny` setting can verify what the Worker described.
The obligation-retention question is real but is a **coverage gap, not a defect in the config**;
recorded under "Coverage gaps" as item 6. The comment in `deny.toml` states the scope accurately
("this only relaxes the check for us, not for third-party dependencies below") - verified against
the file.

## Unit 10c - WP-72 `a5d6f102` confirmed, no finding

10b's verdict stands and is not re-derived: no spec, no checklist row, one-line commit message,
self-certifying. The only check available is whether the comment's factual claims are true. They
are, verified directly:

- `rust/deny.toml:51-56` is a six-line comment plus a trailing `#`, recording `combine` 4.6 as an
  accepted risk with a named future trigger (D-24, "when the markup parser is next rewritten").
- **Claim "combine 4.6 (`rust/lib/markup/Cargo.toml`)"**: true - `rust/lib/markup/Cargo.toml:10`
  is `combine = "4.6.7"`, and it is the only `combine` declaration in the workspace.
- **Claim "no `ignore` entry, no ban"**: true - `combine` appears nowhere in `[advisories].ignore`
  (two entries, both leptos-transitive) nor in `[bans].skip` (24 entries, none `combine`).
- **Claim "carries no advisory today"**: not verifiable without running `cargo deny`, which the
  hard constraint forbids. Recorded as unverified rather than asserted.

The comment is accurate as far as it can be checked. **The finding is the shape of the work
package, not its content**, and that is already recorded as Coverage gap 2.

## Verified good

### WP-64 `4fb252da` - all four headline hunts came back negative

This unit was specifically briefed to hunt, in WP-64, for pattern 2, a silent default change from
a version bump, a feature added or dropped as a side effect, and pattern 4b. **All four are
REFUTED with evidence. Do not re-derive them.**

- **Pattern 2: NOT FOUND.** Every non-`workspace = true` dependency declaration across all
  `rust/**/Cargo.toml` at HEAD was enumerated. **Zero divergent pins remain for any key that
  exists in `[workspace.dependencies]`** - there is no crate holding a private version of a
  hoisted dep. The complementary direction was also checked: key frequency was counted over all
  40 pre-image manifests, and the set of keys with 2+ consumers is *exactly* the hoisted set plus
  `sqlx` and `getrandom`, both explicitly deferred by spec §3b (and `sqlx` subsequently hoisted by
  WP-66). Nothing hoistable was left behind.
- **`warp` is a spec-list error, not a WP-64 omission.** The spec's hoist list names `warp`, but
  it had exactly **one** consumer (`rust/lib/cmd/Cargo.toml:17,25,30` in the pre-image), so
  hoisting it would have been wrong. Not hoisting it was correct.
- **Feature change: exactly ONE in the whole commit, and it is a WIDENING, not a narrowing.**
  `rust/tools/fuzz` went from `serde = "1.0.228"` (no `derive`) to inheriting the workspace
  entry's `["derive"]`. Inert: serde feature-unifies workspace-wide, 35 sibling members already
  enable `derive`, and `brdgme_game` - which `tools/fuzz` itself depends on - is one of them.
  Rider 6 explicitly sanctions this option. **No member had a `default-features = false` that the
  workspace entry fails to carry.**
- **Silent default change from a version bump: NONE.** Eleven version strings changed in
  `4fb252da`; every one is a semver-compatible patch or minor step and **none crosses a 0.x-minor
  or a major boundary**, which is the only place a Cargo default can move.
- **Weakened lints: NONE, and there was nothing to weaken.**
  `git grep -E '#!\[(deny|warn|allow)' 4fb252da^ -- rust/` returns **zero hits**. No crate had a
  stricter attribute-based lint config that the workspace table displaced. (The converse gap - the
  missing `[workspace.lints.rust]` table - is F-203.)
- **Pattern 4b: NOT PRESENT, structurally impossible here.** `4fb252da` touches **zero `.rs`
  files**, so no test or doc could have been edited to agree with code. Every `#[allow(...)]` at
  HEAD was traced to its introducing commit; all predate `4fb252da` or belong to unrelated work
  packages (WP-06, WP-47, WP-52). **No `#[allow(dead_code)]` was added to make the new lints
  pass.**

### WP-70 `8304baf5` (`serde_yaml` -> `serde_yaml_ng`) - the fork risk is REFUTED by direct diff

`serde_yaml_ng` is a fork, so "drop-in" was an assumption worth testing rather than accepting.
It was tested: a full `diff -ru` of both crates' `src/` trees.

- **After the crate rename, the only non-doc differences are `i64::max_value()` -> `i64::MAX` and
  an additive `singleton_map` API.** `ser.rs` differs by a single doc line. The **error type,
  `to_string`, tag handling, anchor handling and `Value` ordering are identical.** There is no
  behavioural difference for any call shape this workspace uses.
- **Call-site coverage is complete, not sampled.** All **7** `serde_yaml_ng` sites at HEAD were in
  the commit's diff: the `[workspace.dependencies]` entry, two consuming manifests, and the four
  code sites `rust/bot/src/prompt.rs:98,105` and `rust/lib/game_client/src/lib.rs:46,299`. **There
  is no third consumer and no deserialise path at all** - every site is serialisation only, which
  is the half of a YAML library least exposed to fork divergence.
- No pattern-2 sibling was left on the old crate: `serde_yaml` appears in no manifest at HEAD.

### WP-69 `e2ee5342` - what did land correctly (the counterweight to F-206)

- **Rider 3 was wrong and the implementer was right to deviate.** The rider asserted
  `wildcards = "deny"` is "free"; it is not, because `cargo-deny` treats a version-less path
  dependency as a `*` wildcard, so every first-party path dep would have failed. The unspecified
  `allow-wildcard-paths = true` (`rust/deny.toml:62-66`) is the correct fix, and its comment
  states the reasoning accurately. The precondition it relies on holds:
  `rust/Cargo.toml:52` sets `publish = false` at workspace level and all 43 other members inherit
  it. **This is a deviation from spec that improves on the spec - recorded so the unified report
  does not mistake it for a shortcut.**
- `wildcards = "deny"` still bites on a real `version = "*"` registry requirement, so the
  criterion's actual intent survives the deviation.

### `22b68689` - correct, and it is the remediation for a real directive

The commit adds `cargo-deny` to `devenv.nix:31`. This is exactly what `docs/CODING.md:547-553`
requires ("All project tooling lives in `devenv.nix`... never install anything globally or ad hoc";
CI may install ad hoc because runners are ephemeral). Without it, the `cargo deny check` that
WP-69 made load-bearing would have had no local invocation path on a NixOS dev machine. Correct
and complete; no finding.

### WP-67 `634c72db` - the lockfile being unchanged is right, not stale

The `sentry` feature trim changed no resolved versions, so `rust/Cargo.lock` correctly needed no
edit in that commit. This was checked rather than assumed, because "a dependency commit that
doesn't touch the lockfile" is normally a smell. (The *substance* of WP-67 is F-205.)

### `be185ccb` - a bookkeeping error only, and a harmless one

`be185ccb` (18:59:27) records WP-69's status as `UNCOMMITTED` **15 seconds after** `e2ee5342`
(18:59:12) committed it - the only row in that table with no SHA. The commit itself was properly
authorised by the EXCEPTION clause present in the same diff, so this is a stale status cell, not
an unauthorised change. **Explicitly not filed as pattern 4b**: the doc was not edited to agree
with wrong code; it simply lost a race with its own subject.

### WP-73 `22d00b8d` - the mass rename is genuinely mechanical, proved not asserted

`00-breakdown.md` said to *sample* 4-5 crates. It was checked **exhaustively** instead, because
sampling cannot answer "did one crate diverge". Method: for each of the 108 pre-commit
`rust/game/*/src/bin/*.rs` files, substitute the crate ident with a placeholder, sort lines to
normalise `use` ordering, hash. Result: **exactly four distinct normalised contents, 27 each**
(`cli`, `http`, `fuzz`, `repl`). **No crate had extra setup, a logging init, a panic hook, a
different port or a different `Gamer` type.** The collapse to 3-line wrappers over
`rust/lib/game_bin/src/lib.rs` is faithful.

Call-site semantics, checked individually against `rust/lib/game_bin/src/lib.rs`:

| Aspect | Old per-game bin | `game_bin` | Changed? |
|---|---|---|---|
| Env var | `ADDR` | `ADDR` (`:27`) | no |
| Default addr | `0.0.0.0:80` | `0.0.0.0:8080` (`:28`) | **yes, deliberate (`e F46`)** |
| Parse/expect | `.expect("Invalid socket address")` | same (`:30`) | no |
| Serve call | `http::serve::<Game>(addr).await` | same (`:31`) | no |
| tokio runtime | `#[tokio::main] async fn main` | `#[tokio::main] async fn http_main_inner` wrapped in a sync `http_main` (`:21-26`) | no - `tokio::main` expands to a sync fn that builds the runtime and `block_on`s; the extra sync wrapper is a no-op |
| CLI stdin/stdout | identical | `:13-19` | no |
| Fuzz entry | `brdgme_fuzz::fuzz_gamer::<G>()` | same (`:35`) | no |
| `*_repl` bin | present | **deleted, no replacement** | **yes, deliberate (D-41)** |

- **The port change is inert in production, verified on the deployment side.** All 43 game
  Deployments under `k8s/base/game/*/deployment.yaml` set `ADDR` explicitly; **all 43 use
  `"0.0.0.0:8080"` with `containerPort: 8080`** (0 deployments rely on the default, 1 distinct
  value). The old `:80` default was the *mismatched* one. This is a fix, not a regression - and it
  is the exact shape of change ("a mechanical diff where a default silently moved") that this unit
  was told to hunt, so it is recorded as checked-and-clean rather than omitted.
- **`[lints] workspace = true` is present on all 44 workspace members**, including the new
  `rust/lib/game_bin` and the later `hanamikoji-1`. WP-64's `[workspace.lints]` table is therefore
  actually in effect - it would have been a silent no-op on any member that omitted the opt-in.
  No gaps.
- **`brdgme_fuzz` in the production game-binary graph is pre-existing, not introduced by WP-73.**
  `git show 22d00b8d^:rust/game/acquire-1/Cargo.toml:10` already had it under `[dependencies]`.
  WP-73 moved it one hop (game crate -> `game_bin` -> `fuzz`). `brdgme_cmd/test-support` is not
  enabled anywhere on this chain (only the 28 `[dev-dependencies]` entries, and `resolver = "2"`
  keeps dev-dep features out of `cargo build`) - consistent with 10a's obligation-5 discharge.
- No game crate declares `[[bin]]` on either side; `rust/Dockerfile` copies `<crate>_http` names
  that did not change, so it correctly needed no edit.
- tokio features narrowed from `["full"]` (per game crate) to `["macros","rt-multi-thread"]` in
  `rust/lib/game_bin/Cargo.toml:13`, unioned with `brdgme_cmd`'s `["signal","net","rt"]`. That
  covers everything actually used (`tokio::signal::unix` at `rust/lib/cmd/src/http.rs:9`,
  `tokio::net::TcpListener` at `:74`). A correct tightening, and it removes the workspace's only
  `tokio/full` feature-union source.
- **The deleted `*_repl` binaries are a capability *move*, not a loss - REFUTED as a finding.**
  `rust/tools/repl` survives as a workspace member (`rust/Cargo.toml:44`); its 10-line `main`
  (`rust/tools/repl/src/main.rs:6-10`) drives *any* game through
  `brdgme_cmd::requester::parse_args`, whose `local` arm
  (`rust/lib/cmd/src/requester/mod.rs:18-26`) spawns a game's `_cli` binary. The generic tool
  strictly supersedes the 27 static copies. Nothing in `docker-bake.hcl`, `rust/Dockerfile`,
  `Tiltfile`, `k8s/` or `scripts/` referenced a `*_repl` binary.
  **A Worker reported `docs/porting/GAME_PORTING.md` documenting a non-existent package
  `brdgmen`; I checked the file directly and this is WRONG.** `GAME_PORTING.md:215` reads
  `cargo run -p brdgme_repl -- local target/release/<name>_N_cli`, which matches
  `rust/tools/repl/Cargo.toml:2` exactly. **Do not carry this into the unified report.**

### WP-65 `2c28ae85` - the five rows that did land correctly

- **`dp F4`**: `rust/web/Cargo.toml` has no `[profile.*]` table at HEAD; the authoritative copy
  survives at `rust/Cargo.toml:88-94`, and both consumers resolve against it -
  `rust/web/Cargo.toml:188` (`lib-profile-release = "wasm-release"`) and `rust/Dockerfile:96`
  (`target/front/wasm32-unknown-unknown/wasm-release/web.wasm`). Consistent.
- **`dp F5`**: `rust/Cargo.toml:2-46` members are strictly lexicographic; `rg 'android-dev|server-dev'`
  over the repo returns **zero hits** - neither profile is defined or referenced anywhere.
- **`dp F17`**: correct, and the gating is one level *above* the call site, which is the part
  worth stating. `rust/lib/cmd/src/http.rs:54` calls `env_logger::init()` un-annotated, but
  `rust/lib/cmd/src/lib.rs:10-11` is `#[cfg(feature = "http-server")] pub mod http;`, so
  `--no-default-features` compiles the whole module out and `env_logger` is never named. The
  build is sound with the feature off. No deployable loses logging: `bot`, `operator` and `web`
  each initialise `tracing_subscriber` themselves, and `rust/lib/cmd/src/` has **zero** `log::`
  call sites.
  Caveat worth recording: all 28 game crates pull `brdgme_cmd` with default features via
  `rust/lib/game_bin/Cargo.toml:9`, so a workspace-wide `cargo build` unifies `http-server` back
  on. The `default-features = false` in bot/operator/web only bites on a per-package build -
  which is what `rust/Dockerfile` does, so the saving is real where it matters.
- **`e F9`**: exactly one `mod test` survives workspace-wide -
  `rust/game/lost-cities-1/src/lib.rs:682` - which is the crate the commit message says it
  deliberately skipped. 122 files carry `mod tests`. Row satisfied exactly as claimed.
- **The three dispositioned rows check out**: `dp F21` (`lazy_static` in `lib/color`) is genuinely
  already resolved - `rust/lib/color/Cargo.toml` has no `lazy_static` key at HEAD. `dp F22` and
  `dp F9` are recorded as monitor-only/parked in the commit message rather than silently dropped.

## Coverage gaps

1. **Nothing in this unit is covered by a test, and that is by design, not falsification.**
   WP-65's nine checklist rows are all `Test? = n`; WP-64/66/67/69/70/73 have no checklist row at
   all (explicitly deferred as BLOCKED-ON-DECISION D-19/D-20/D-23); WP-72 appears in no checklist
   and has no spec. **The unified report must count these with WP-76/77/79/80 as
   "untested by design", NOT toward the nine-strong "Test? y with no test" tally.**
2. **WP-72 has no spec, no checklist row and a one-line commit message.** It is a seven-line
   comment in `rust/deny.toml:51-56` recording `combine` 4.6 as an accepted risk. There is
   literally no acceptance criterion to check it against - the WP is self-certifying. Worth
   naming in the process-fixes section: a work package that exists only as a commit cannot be
   verified by any sign-off procedure.
3. **`deny.toml`'s `skip` list is unverifiable by reading.** **Correction from 10c: it has 29
   entries, not 24** (`rust/deny.toml:77-138`); the earlier count was low. Each entry names (a) the
   crate pulling the extra copy and (b) the upstream change that would clear it, but nothing
   re-checks those claims - a `skip` entry silently outlives its cause, and `cargo deny` does not
   warn on an unused skip unless configured to. There is no expiry, no `unused-skip` setting, and
   the weekly job (F-199) does not run `bans` at all. This list will rot. **10c adds the decisive
   point: WP-69's spec §5 negative checks ("the flip must actually bite") are recorded in
   `EXECUTION-STATE.md` as parked, never run** - so nothing has ever demonstrated that
   `multiple-versions = "deny"` rejects a new duplicate rather than being neutered by the skip
   list. See F-206.
3b. **The `[advisories].ignore` list has the same shape and the same absent expiry.** Two entries
   (`rust/deny.toml:16,18`, both leptos-transitive `unmaintained` notices) with a stated reason but
   no review date, and the weekly job that *does* run `advisories` (F-199) can never alert anyone.
4. **Nothing tests the vendored `rust/lib/session_store`.** It has no `tests/` directory and no
   `#[cfg(test)]` module; the session layer is authentication-adjacent and is now first-party
   code carrying first-party maintenance obligations.
5. **The vendored MIT obligations are satisfied by hand and cannot be machine-checked.**
   `rust/lib/session_store/LICENSE` (MIT, Max Countryman 2023) and the attribution header at
   `src/lib.rs:1-3` are the only things discharging the MIT redistribution obligation for
   `brdgme_session_store`, and nothing in CI asserts either still exists. This is inherent to
   vendoring (see the `deny.toml` disposition above), not a misconfiguration. A one-line CI grep
   for the LICENSE file plus the header would close it. Pairs with F-200's recommendation that a
   vendoring spec carry explicit inherited-obligation criteria.
6. **CARRY-FORWARD TO UNIT 11 (found incidentally while reading `rust/Dockerfile` for WP-73/64
   deployment impact): `hanamikoji-1` has no Docker stage and therefore cannot be deployed.**
   `rust/Dockerfile:174-303` defines exactly **26** game runtime stages. `rust/Cargo.toml:4-31`
   lists **28** game members. The two with no stage are `lords-of-vegas-1` (WIP crate, excluded by
   owner ruling) and **`hanamikoji-1`**. The crate is a full workspace member with a
   `brdgme_game_bin` dependency (`rust/game/hanamikoji-1/Cargo.toml:9`), so
   `hanamikoji_1_http` **is** built by `cargo build --release --workspace --exclude web`
   (`rust/Dockerfile:36`) and then never copied into an image. Unit 11 should also check
   `docker-bake.hcl` and `k8s/base/game/` for the same gap. Not filed as a 10b/10c finding -
   `hanamikoji-1` is Unit 11's crate.
8. **`docs/reviews/2026-07-23-rust-review/SUMMARY.md:44-46,139` and the archived
   `findings/dependencies.md:103-108,157` assert a dependency fact that is false** (sentry pulling
   actix-web/ureq into every build). See F-205. The unified report should amend the corpus entry,
   not just record the WP as closed.
7. **Stale docs contradicting the code at HEAD** (not findings, but sign-off noise):
   `docs/changes/archive/2026-07-16-42-image-scale-to-zero-viability-spec/spec.md:14` and
   `docs/changes/archive/2026-07-16-42-image-scale-to-zero-plan/plan.md:41` both still assert the game
   HTTP default is `0.0.0.0:80` and cite `no_thanks_2_http.rs`, a path WP-73 deleted.

## Carry-forwards from Unit 10c (for the unified report and Unit 11)

- **F-206 is the unit's headline and belongs in the process-fixes section as a new pattern:
  *a spec's own escalation trigger fired and the implementation answered it with a comment.***
  This is a step beyond pattern 4b (tests/docs edited to agree with the code) and beyond 4c (the
  acceptance criterion quietly renegotiated): here the criterion was a **stop-work condition**,
  and the artefact it governed was edited to pre-argue against it. Remediate together with F-199
  and Coverage gap 3 - they are three views of one unenforced `bans` section.
- **F-205 is a second new pattern: *the finding whose premise was disproved, closed anyway,
  never amended.*** Sign-off fix: when implementation disproves a finding's stated mechanism, the
  finding must be **amended**, not merely closed. WP-67's spec actually required this (rider 2)
  and it still did not happen, so the fix is a procedural check, not another rider.
- **Untested-by-design tally, final for Unit 10.** WP-64/66/67/69/70/73 have no checklist row;
  WP-65's nine rows are all `Test? = n`; WP-72 appears in no checklist and has no spec.
  **No Unit 10 row counts toward the nine-strong "Test? y with no test" tally.** Group with
  WP-76/77/79/80. This is now settled and should not be re-derived.
- **REFUTED in 10c, do not re-derive**: (1) WP-64 has no pattern 2, no silent default change, no
  feature narrowing and no pattern 4b - all four proved negative, not merely unfound; (2) the
  WP-66 `default-features = false` sqlx narrowing is inert (compile-time-only features, zero
  affected call sites); (3) `serde_yaml_ng` is a faithful fork for every shape this workspace
  uses, by full source diff; (4) `[licenses.private] ignore = true` is correct configuration and
  no `cargo-deny` setting could machine-check the vendored MIT obligations; (5) `22b68689` and
  WP-69's `allow-wildcard-paths` deviation are both correct.
- **For Unit 11**: `hanamikoji-1` has **no `rust/Dockerfile` stage** (Coverage gap 6) - check
  `docker-bake.hcl` and `k8s/base/game/` for the same gap. Note 10b recorded **43** game
  Deployments under `k8s/base/game/` against only **26** Dockerfile stages, so the k8s side and
  the image side already disagree in count; Unit 11 should establish which list `hanamikoji-1` is
  on.
- **Remediation pairing**: F-207 (sqlx-cli 0.8.6 in prod vs unpinned in CI) belongs with the
  F-96-family deployment checklist, not with the code findings - it is a build-tooling item.

## Handover - NO Unit 10d is required, Unit 10 is CLOSED

Unit 10c completed its full scope before hitting the budget line. Recorded here so a successor
does not re-open closed work.

| Item | Status | Evidence location |
|---|---|---|
| WP-66 wrap-up (both open items) | **DONE** - `deny.toml [licenses.private]` benign, `default-features = false` REFUTED | "Unit 10c - WP-66 wrap-up" |
| WP-64 `4fb252da` | **DONE** - all of spec §3a-3c + riders 1-6 | F-203, F-204, "Verified good" |
| WP-67 `634c72db` | **DONE** | F-205, "Verified good" |
| WP-69 `e2ee5342` | **DONE** - all 29 skip entries checked individually | F-206 |
| WP-69 `be185ccb` (docs) | **DONE** - bookkeeping error only, explicitly NOT pattern 4b | "Verified good" |
| WP-70 `8304baf5` | **DONE** - fork risk refuted by full source diff, all 7 sites | "Verified good" |
| WP-72 `a5d6f102` | **DONE** - comment claims verified true | "Unit 10c - WP-72" |
| `22b68689` | **DONE** - correct, no finding | "Verified good" |
| Q1 sqlx-cli split | **DONE** | F-207 |
| Q2 `deny.toml` self-contradiction | **DONE** - folded into F-206 | F-206 |

- **Next finding number: F-208.** Unit 11 continues from there.
- **Nothing is in flight and no lead is left dangling.** The only deliberately unresolved items
  are the two that the hard constraint makes unresolvable without running `cargo`: whether any
  `deny.toml` skip entry is already stale, and whether sqlx 0.9 changed the `_sqlx_migrations`
  format. Both are recorded as UNVERIFIED in F-206 and F-207 respectively, with the reason.
- **Expensive negatives a successor must not reproduce** are listed under "Carry-forwards from
  Unit 10c" above and mirrored into `00-STATE.md`. The costliest were the WP-64 four-way
  refutation (required enumerating every dependency declaration across 44 manifests plus all 40
  pre-image manifests) and the WP-70 fork refutation (required a full `diff -ru` of two crate
  source trees).
