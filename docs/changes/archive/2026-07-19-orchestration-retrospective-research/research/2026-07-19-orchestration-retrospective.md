# Orchestration Retrospective - #29 Profile Pages Session (2026-07-18/19)

Durable learnings from the orchestrated session that shipped feature #29
(player profile pages) in 9 commits (daf5eef..5bed080) via an
Orchestrator -> Leads -> Workers hierarchy. Recorded here because the coree
memory system was unavailable in-session, and `~/.claude/skills/` is a
read-only Nix mount - the "Proposed orchestrate skill additions" section at
the end is ready to paste into the home-manager source for that skill.

## What worked - keep doing

- **A survey unit (L0) first.** One small Lead produced
  `2026-07-18-29-profile-pages-context.md` (codebase map, schema facts,
  hydration gotchas, sqlx workflow, v1 decisions) before any implementation.
  Every later Lead followed it; zero hydration issues and no pattern rework
  across 7 implementation units. Cost: ~60k tokens. This should be the
  default opening unit for any multi-Lead feature.
- **Unit sizing at "one page section / one layer".** Units of roughly one
  data layer, one component set, or one page section each fit comfortably in
  a Lead + 2-4 Workers. Nothing hit the 150k budget through overwork; the
  only failures were external (usage limits).
- **Per-unit commits gated on verification.** Each unit ended with the same
  gate - clippy `-D warnings` (ssr), `cargo test -p web --features ssr`,
  wasm hydrate check, `cargo sqlx prepare --check`, fmt on touched files -
  then a single conventional commit. Push deferred to the final unit. Made
  lead deaths cheap to recover from: completed work was already on master.
- **Leads reviewing Worker SQL.** A Lead reading a Worker's query caught a
  real bug the tests missed: `FULL OUTER JOIN game_type_users ... AND
  gtu.user_id = $1` leaks other users' rows as unmatched right-side rows
  (user filter in the join condition does not filter a FULL OUTER JOIN).
  Fix: pre-filter in a subquery, then join. Regression test added. Pattern:
  user-scoped columns in FULL/RIGHT OUTER JOINs must be filtered in a
  subquery or WHERE, never only in the ON clause.
- **Known-issue lists flowing between briefs.** Carrying "stats/queries.rs +
  stats/viz.rs are fmt offenders, leave them" and "2 NATS tests are
  known-flaky when Tilt is down" through every brief stopped Workers from
  "fixing" out-of-scope noise or misreading test results.
- **Scope overrides via task descriptions.** When the product owner's brief
  overrode the spec (active games public, not own-profile-only), the
  Orchestrator recorded it in the pending task's description immediately so
  the change survived context loss and reached the right Lead verbatim.

## What failed - and the countermeasures

- **Leads idling on "waiting for worker notification".** The most common
  failure by far: Leads stopped their turn to "wait", which halts all work
  until the Orchestrator nudges. Countermeasure that worked: put "do not
  stop to wait for notifications - read worker output files directly and
  keep driving" in every Lead brief. Later Leads with that line still
  occasionally idled, but recovery was one nudge instead of a stall.
- **Leads dying at usage limits.** Recovery pattern that worked well: do NOT
  resume the dead (now uncached) agent. Spawn a fresh Lead whose brief
  contains a precise state summary (what is done, verified, uncommitted;
  exact remaining deliverables). Both recoveries (L1 -> L1b, L7 -> small
  commit agent) were clean. Corollary: have Workers verify and Leads commit
  as they go, so the state summary is short.
- **Workers misrouting reports.** Detached Workers sometimes reported to the
  main session instead of their Lead ("no reachable agent named claude").
  The Orchestrator must treat these as routine: relay the report to the
  owning Lead via SendMessage and let it proceed - do not act on the
  Worker's behalf beyond relaying.
- **`cargo fmt -p web` reformats the whole package.** A Worker trying to
  format one file with `cargo fmt -p web -- <file>` reformatted everything,
  touching out-of-scope files; `git checkout` was blocked by the permission
  sandbox and recovery needed hunk-by-hunk reverts against
  `git show HEAD:<path>`. Rule: format single files with
  `rustfmt --edition 2024 <file>`, never `cargo fmt -p`.

## Repo-specific facts confirmed this session

- The `docs/hydration.md` + CODING.md data-loading pattern
  (`Resource::new_blocking` + Suspense) works: first-ever use of blocking
  resources on the profile pages produced zero hydration panics.
- `cargo sqlx prepare` regen routinely captures more query JSONs than the
  lines you changed suggest - the macro cache is per-query-shape. Always
  re-run `--check` after regen and expect multi-file .sqlx diffs; commit
  them with the unit.
- CI-equivalent local gate for `rust/web`:
  `cargo fmt --all -- --check`;
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`;
  `cargo test -p web --features ssr`;
  `cargo check -p web --target wasm32-unknown-unknown --features hydrate`;
  from `rust/web`: `cargo sqlx prepare --check -- --tests --features ssr
  --all-targets`.
- `rust/web/tests/ssr_pages.rs` has a reusable `spawn_mock_game_service`
  pattern plus cookie-based viewer fixtures - use it for any page test that
  needs a live-looking game or a logged-in viewer; do not invent new test
  infra.
- 2 NATS game tests are ignored/flaky unless the Tilt/Kind dev stack is
  running; not a regression signal.

## Proposed orchestrate skill additions

Paste into the home-manager source for `~/.claude/skills/orchestrate/SKILL.md`
(the mount is read-only in sessions):

```markdown
## Delegating (additions)

- For multi-Lead features, make the first unit a small survey Lead that
  writes a context-handover doc (codebase map, patterns to copy, gotchas,
  decisions resolving spec gaps) into the repo's planning docs. Every
  later Lead brief points at it.
- Every Lead brief should include: the handover doc and prior units'
  outputs to read; exact deliverables; "do not stop to wait for worker
  notifications - read worker output files directly and keep driving";
  the verification gate to run before committing; the carry-forward
  known-issues list; and the report format expected back.
- Each unit ends with verification green and a single commit. Defer
  pushing to a final cleanup unit. Committed units make failures cheap.

## Failure handling

- Lead stops mid-unit to "wait" on a worker: nudge it once to read the
  worker's output and keep driving. This is the most common stall mode.
- Lead dies (usage limit, crash): do not resume it - its cache is gone.
  Spawn a fresh Lead whose brief contains a precise state summary: done /
  verified / uncommitted / exact remaining deliverables. If only a
  mechanical step remains, a single Worker with that step is fine.
- Worker reports to the wrong session: relay the report to the owning
  Lead and let it proceed; don't absorb the Lead's job.
- Scope changes from the user mid-run: record them immediately in the
  pending task's description so they survive context loss and reach the
  right Lead verbatim.
```
