# WP-69 (+ WP-72): `deny.toml` hardening and the `combine` posture note

> **WP-72 has no spec file of its own - it is section 3d below.** D-24 reduced
> WP-72 to "add a comment to `deny.toml`". Do not look for a `WP-72-*.md`.

**Findings:** dp F18, dp F24, dp F25, dp F15 (all minor).
**Decisions:** D-23, D-24.

**Landing order:** WP-69 lands **LAST** among the dependency packages, after
WP-64/66/67/68, so the `skip` list starts minimal. Section 3a (clearing stale
advisory ignores) is ungated and may land at any time.

> **Read every named file/table/key before editing. No line numbers are cited
> on purpose; the tree is under concurrent edit. If a file does not match what
> this spec describes, STOP and report rather than improvising.**

## 0. Step 0 - upgrade to latest FIRST (binding, do not skip)

Michael's standing strategy is to stay as close to latest as possible so deps
never go stale. First step: **"upgrade all dependencies to latest and see where
we stand"**. Here it is literal - the `skip`/`skip-tree` list in 3b must be
derived from the lock **after** the upgrade sweep and after WP-66/67/68 land,
never from the findings' snapshot. **If the upgrade plus the siblings leave zero
duplicates, 3b becomes `multiple-versions = "deny"` with empty
`skip`/`skip-tree`** and the enumeration work disappears.

## 1. Problem

Config is `/home/beefsack/Development/brdgme/rust/deny.toml` (confirmed live;
no repo-root copy). CI runs `cargo deny check` with working-directory `rust`.

- **dp F25** - `[advisories].ignore` has 7 entries; 4 cite a "legacy rust/api"
  crate for diesel 1.4.8 (RUSTSEC-2024-0365, -2026-0136, -2026-0137) and
  `encoding` (RUSTSEC-2021-0153).
- **dp F18 / dp F24** - `[bans] multiple-versions = "warn"`, empty `skip` and
  `skip-tree`, `wildcards = "allow"`; `[sources] unknown-registry` and
  `unknown-git` both `"warn"`. Nothing gates.
- **dp F15 (WP-72)** - dormant `combine 4.6` backs `brdgme_markup`'s parser and
  parts of `brdgme_game`.

## 2. Why it's wrong

- **dp F25 is correct - verified live.** `diesel` and `encoding` both have grep
  count **0** in `rust/Cargo.lock`, and the `members` array in `rust/Cargo.toml`
  (40 members) contains **no `api`**. All four are dead, and since cargo-deny
  reports ignores that never matched they already produce `unmatched-ignore`
  noise - part of why they go.
- **dp F18/dp F24 are correct on the current values**, and its "no member uses
  a wildcard req today" claim is correct: zero `= "*"` / `version = "*"` across
  all manifests under `rust/`, so `wildcards = "deny"` is free. **Its timing is
  wrong for us**, though - flipping `multiple-versions` to `deny` now would fail
  CI immediately (2x sqlx, 3x rand, 3x getrandom are live). D-23 corrects it:
  flip only after the siblings land.
- **dp F15 is correct and D-24 accepts it.** No advisory against `combine`
  today; WP-02 already changes markup enough for one release.

## 3. Required end state - `rust/deny.toml`

### 3a. Clear the 4 stale advisory ignores (ungated, do first)

Delete from `[advisories].ignore` the entries `RUSTSEC-2024-0365`,
`RUSTSEC-2026-0136`, `RUSTSEC-2026-0137`, `RUSTSEC-2021-0153`, **together with
the two comment blocks above them** mentioning diesel/`r2d2-diesel` and
`encoding`/legacy `rust/api`. Leave every other entry alone. Accounting (agrees
with `planning/specs/WP-68-term-size-replacement.md`, which owns one removal):

| Entry | Owner | End state |
|---|---|---|
| RUSTSEC-2024-0365 / -2026-0136 / -2026-0137 (diesel) | **WP-69 (3a)** | removed |
| RUSTSEC-2021-0153 (encoding) | **WP-69 (3a)** | removed |
| RUSTSEC-2020-0163 (term_size) | **WP-68** - do not touch here | removed by WP-68 |
| RUSTSEC-2024-0436 (paste, via leptos) | nobody | **kept** |
| RUSTSEC-2026-0173 (proc-macro-error2, via leptos_macro) | nobody | **kept** |

7 today, **2** once WP-68 and WP-69 both land. If the live file has a different
count when you open it, STOP and report.

### 3b. `[bans]` - flip to deny (gated on WP-64/66/67/68 having landed)

- `multiple-versions = "deny"`, `wildcards = "deny"`. Leave `highlight`,
  `workspace-default-features`, `external-default-features` and
  `allow-workspace` as they are.
- `skip` / `skip-tree`: enumerate residual duplicates from a fresh
  `cargo tree -d` taken **after** the siblings land. **Every entry must carry
  an inline comment naming (a) which crate pulls the older copy and (b) what
  upstream change would remove it** - e.g. rand 0.9 via leptos/governor/
  tungstenite, rand 0.8 via nkeys+nuid (async-nats). Prefer `skip` (single
  crate+version) over `skip-tree`.
- **Forbidden:** any blanket or wildcard skip, a `skip-tree` on a first-party
  `brdgme_*` crate, and any entry without a comment. If the residual list runs
  past roughly a dozen entries, a sibling package did not do its job - STOP and
  report rather than papering over it.

### 3c. `[sources]`

Set `unknown-registry = "deny"` and `unknown-git = "deny"`. Leave
`allow-registry` as the single crates.io index and `allow-git` empty.

**Interaction with WP-66:** if WP-66 vendored, the store at
`rust/lib/session_store/` is a **`path`** dependency. Workspace path deps are
neither a registry nor a git source, so they do not trip `[sources]` and need
no allow-list entry; `[licenses.private] ignore = true` already covers
`publish = false` members, so no licence entry either. If `cargo deny check`
complains anyway, STOP and report.

### 3d. WP-72 - record `combine` as an accepted risk (comment only)

Add a comment block to `rust/deny.toml` (above `[bans]` is fine) recording that
`combine 4.6` (`rust/lib/markup/Cargo.toml`, `rust/lib/game/Cargo.toml`) is a
knowingly dormant dependency, carries no advisory today, and is to be migrated
(winnow, or folded into the in-house combinator in `lib/game`) only when the
markup parser is next rewritten - per D-24, because WP-02 already changes markup
enough for one release. **No `ignore` entry, no ban, no code change, no manifest
change.** That is all of WP-72.

## 4. Non-goals

- Removing the RUSTSEC-2020-0163 `term_size` ignore - WP-68 owns it, in the
  same commit as its code swap.
- Any change under `rust/**/src/` or to any `Cargo.toml`; touching
  `[licenses]`, `[graph]`, `[output]`.
- Fixing duplicates. WP-69 only records what WP-64/66/67/68 left behind.

## 5. Regression test cases

- `cargo deny check` (from `/home/beefsack/Development/brdgme/rust`) passes
  clean - **including no `unmatched-ignore`/"advisory not encountered"
  warnings**, which is the direct test that 3a's removals were right.
- `cargo deny check advisories`, `... bans`, `... sources`, `... licenses` each
  clean individually.
- Negative check: temporarily add a wildcard req to a scratch manifest and
  confirm `cargo deny check bans` now **fails**; revert. Same for a deliberately
  duplicated version. The flip must actually bite.
- Review `skip` against `cargo tree -d`: every skipped duplicate still exists
  (no stale skips) and every remaining duplicate is skipped. Stale skips are
  the same defect as the stale ignores in 3a.
- No Rust changes, so no per-crate `cargo check` needed. Do **not** run a
  workspace-wide `cargo build`/`test`/`clippy` (AGENTS.md "Resource
  constraints"). Final gate:
  `/home/beefsack/Development/brdgme/scripts/rust-test.sh`.

## 6. Riders

| # | Item | Source |
|---|------|--------|
| 1 | Exactly 4 ignore entries removed, with their comment blocks | dp F25 |
| 2 | 2 ignores remain after WP-68 + WP-69 (paste, proc-macro-error2) | dp F25 / WP-68 |
| 3 | `wildcards = "deny"` (verified free: no wildcard reqs in any manifest) | dp F24 |
| 4 | `unknown-registry` and `unknown-git` both `"deny"` | dp F24 |
| 5 | Every `skip`/`skip-tree` entry commented with cause and exit condition; no blanket skips | D-23 |
| 6 | `combine` accepted-risk comment present; no `combine` code or manifest change | dp F15 / D-24 |
