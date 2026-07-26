# WP-66: sqlx unification (0.8 in `web` vs 0.9 in `bot`/`operator`)

**Findings:** dp F6 (major), dp F8 (minor), dp F19 (minor). **Decision:** D-17.

**Landing order:** after WP-64 - `[workspace.dependencies]` exists by then, so
a version change is a one-line root edit. WP-66/WP-67 are independent and may
land in either order. WP-69 lands last.

> **Read every named file/table/function before editing. No line numbers are
> cited on purpose; the tree is under concurrent edit. If a file does not match
> what this spec describes, STOP and report rather than improvising.**

## 0. Step 0 - upgrade to latest FIRST (binding, do not skip)

Michael's standing strategy is to stay as close to latest as possible so deps
never go stale. First step for any dependency problem: **"upgrade all
dependencies to latest and see where we stand"** - the problem may dissolve.
Bump `tower-sessions`, `tower-sessions-sqlx-store` and `sqlx` to latest and
re-resolve before designing anything. **If that alone puts every crate on one
sqlx major, this spec collapses to section 3a and you are done - do NOT vendor
anything.** Only if no sqlx-0.9-compatible store release exists does 3b apply.

## 1. Problem

- **dp F6 (major)** - `rust/web` declares `sqlx = "0.8"`; `rust/bot` and
  `rust/operator` declare `"0.9"`. `rust/Cargo.lock` genuinely carries both
  `sqlx 0.8.6` and `sqlx 0.9.0`. Two type-mapping behaviours, one database.
- **dp F8 (minor)** - `bot` declares `getrandom = "0.3"`, `web` `"0.4"`
  (`wasm_js`); lock carries 0.2/0.3/0.4. **dp F19 (minor)** - rand
  0.8/0.9/0.10 in the lock; all first-party declarations are already 0.10.

## 2. Why it's wrong

- **dp F6 is correct as written, cause guess included.** `rust/web/Cargo.toml`
  also declares `tower-sessions-sqlx-store = { version = "0.15.0", features =
  ["postgres"] }` and `tower-sessions = "0.14.0"`; the store 0.15.0's manifest
  requires `sqlx = "0.8.0"`, which is what holds `web` back.
- **Currency check - re-verify, may have moved.** At 2026-07-26 the store's
  newest is still **0.15.0** (sqlx 0.8, tower-sessions 0.14) while
  `tower-sessions` itself is 0.15.0, so Step 0 likely will not resolve dp F6
  and branch 3b is the live one.
- **dp F8 / dp F19 are correct but WP-66 does not fix them.** Retiring sqlx 0.8
  drops its drivers' rand 0.8 copy, but rand 0.8/0.9 also arrive via nkeys/nuid
  (async-nats), leptos, governor, sentry-core and tungstenite. They are
  **re-audit items** - work-packages calling them monitor items is correct. The
  one first-party action is bot's direct `getrandom` (rider 1).

## 3. Required end state

### 3a. Branch A - Step 0 converged everything on one sqlx major

One `sqlx` entry in `[workspace.dependencies]` in `rust/Cargo.toml`; `web`,
`bot`, `operator` use `sqlx.workspace = true` plus their own features.
**Feature reconciliation is mandatory - versions alone are not enough.** Live
declared sets differ: `web` = `runtime-tokio-rustls`, `postgres`, `uuid`,
`migrate` (optional); `bot` = `runtime-tokio`, `tls-rustls`, `postgres`,
`uuid`, `time`, `json`; `operator` = `runtime-tokio`, `tls-rustls`, `postgres`,
`uuid` (+ `macros`, `migrate` in its `[dev-dependencies]`). Root entry carries
the intersection (`runtime-tokio`, `tls-rustls`, `postgres`, `uuid`);
`migrate`/`time`/`json`/`macros` stay per-crate. `runtime-tokio-rustls` is the
0.8 spelling of `runtime-tokio` + `tls-rustls` and must be respelled. **Do not
switch any crate to native-tls** - the rustls choice is documented in
`rust/operator/Cargo.toml`'s crypto-provider comment and `docs/CODING.md`.

### 3b. Branch B - no sqlx-0.9-compatible store: vendor it

Decide by one check: after Step 0, does the store's published manifest accept
sqlx 0.9? If no, vendor. **There is no third option. Do not leave the split, do
not pin bot/operator back to 0.8, do not run two pools.** Two type-mapping
behaviours against one database is unacceptable; vendoring is preferable.

- New member `rust/lib/session_store/` (package `brdgme_session_store`,
  `publish = false`), consumed by `rust/web/Cargo.toml` as
  `{ path = "../lib/session_store", optional = true }` replacing the
  `tower-sessions-sqlx-store` entry, including its slot in web's `ssr` feature
  list. Add it to the `members` array in `rust/Cargo.toml`; it inherits WP-64's
  `[workspace.package]` and `[lints]`.
- **Minimal port, not a rewrite.** Copy only `src/lib.rs` (50 lines) and
  `src/postgres_store.rs` (266 lines) from the upstream 0.15.0 crate. Drop the
  MySQL and SQLite stores and their features. Change only what sqlx 0.9
  requires. Do not rename `PostgresStore`, change the SQL, or change defaults.
- **Preserve licence and attribution.** Upstream is MIT,
  `https://github.com/maxcountryman/tower-sessions-stores`. Copy the LICENSE
  into `rust/lib/session_store/` and head `src/lib.rs` with a comment naming
  the upstream crate, version 0.15.0, and MIT.
- **The schema must not change.** The store creates schema `tower_sessions`,
  table `session`, via its own `migrate()` - not via `rust/web/migrations/`.
  The vendored `migrate()` must emit identical DDL so existing rows survive.
  There is **no** `continuously_delete_expired` sweeper in `rust/web` today
  (expiry is cookie-driven); the port must neither add one nor drop upstream's
  `delete_expired` method.
- Call site: `create_session_layer` in `rust/web/src/auth/session.rs`
  (`PostgresStore::new(pool.clone())`, `.migrate()`, `SessionManagerLayer` with
  `with_secure`, `SameSite::Lax`, `Expiry::OnInactivity(30 days)`). Only the
  `use` line should change; if anything else must, STOP and report.

## 4. Non-goals

- The workspace tables (WP-64, first); sentry (WP-67); `deny.toml` (WP-69);
  `serde_yaml` (WP-70); `warp` (WP-71).
- Chasing rand/getrandom duplicates sourced from leptos, async-nats, governor,
  sentry or tungstenite - re-audit only. Any change to `rust/web/migrations/`:
  migrations are immutable.

## 5. Regression test cases

- **Never run a bare workspace-wide `cargo build`/`test`/`clippy`** (AGENTS.md
  "Resource constraints"). Use `cargo check -p web --features ssr`,
  `-p web --features hydrate`, `-p bot`, `-p operator`.
- `rust/Cargo.lock` has one `name = "sqlx"` entry;
  `cargo tree -p web --features ssr -i sqlx` shows one version;
  `cargo tree -d` before/after shows the sqlx 0.8 stack gone and no new
  duplicate.
- Behaviour (`cargo test -p web --features ssr` plus the noted manual pass):
  login creates a row in `tower_sessions.session`; a cookie issued before the
  change still authenticates after a process restart (manual: log in, restart,
  reload); `Expiry::OnInactivity(30 days)` still configured; `migrate()` on an
  already-migrated database is a no-op and leaves `\d tower_sessions.session`
  identical before/after.
- `(cd web && cargo sqlx prepare --check -- --tests --features ssr
  --all-targets)` passes - a major bump commonly invalidates the offline cache;
  if it fails, regenerate and commit the `.sqlx/` diff.
- CI clippy split: `cargo clippy --workspace --exclude web --all-targets -- -D
  warnings`, then `cargo clippy -p web --all-targets --features ssr -- -D
  warnings`. Final gate:
  `/home/beefsack/Development/brdgme/scripts/rust-test.sh`.

## 6. Riders

| # | Item | Source |
|---|------|--------|
| 1 | bot's direct `getrandom = "0.3"` bumped to `"0.4"` or dropped for `aes-gcm`'s `generate_nonce`; say which | dp F8 |
| 2 | `runtime-tokio-rustls` respelled `runtime-tokio` + `tls-rustls`; no native-tls in sqlx | dp F6 |
| 3 | If vendoring: MIT text + upstream attribution present; new member added to `members` | D-17 / WP-64 |
