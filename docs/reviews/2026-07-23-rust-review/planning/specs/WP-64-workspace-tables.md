# WP-64: workspace tables (`dependencies` + `package` + `lints`)

**Findings:** dp F1 (major), dp F2 (minor), dp F3 (minor). **Decision:** D-19 answered
option A - all three tables in ONE pass, early.

> **Read every named file/table before editing. No line numbers are cited on
> purpose; the tree is under concurrent edit. If a file does not match what
> this spec describes, STOP and report rather than improvising.**

## 0. Step 0 - upgrade to latest FIRST (binding, do not skip)

Michael's strategy: stay as close to latest as possible so deps never go stale. Step 0
for any dependency problem is **"upgrade all dependencies to latest and see where we
stand"** - bump every direct dep to latest (root + all 40 members), build, record what
changed; the problem may dissolve. **The migration below is still required regardless**
- workspace tables are structural, not a workaround. The ordering exists only so
versions are hoisted at final values, not migrated then bumped twice.

## 1. Problem

- **dp F1** - no `[workspace.dependencies]`; versions copied across 40 manifests.
- **dp F2** - no `[workspace.package]`; metadata x40, `authors` already drifted.
- **dp F3** - no `[lints]`/`[workspace.lints]`; lint policy is per-invocation.

## 2. Why it's wrong

- **dp F1 is correct; counts slightly stale.** Live: zero
  `workspace.dependencies`/`workspace = true` hits; serde 36, tokio 32, rand 32,
  serde_json 19, thiserror 9 manifests (findings said tokio/rand 33); mixed spellings
  confirmed, e.g. tokio `1.52.3` x27 + `1` x8.
- **dp F2 is correct.** 41 manifests (root + 40 members); all 40 carry
  `version = "0.1.0"`, `publish = false`, `edition = "2024"`; `authors` in 37,
  absent from `bot`/`web`/`operator`. No `license`/`repository`/`rust-version`.
- **dp F3 is correct.** No `[lints]` table anywhere and **zero crate-level
  `#![deny]`/`#![warn]`/`#![allow]` attributes** under `rust/`.

## 3. Required end state

### 3a. `rust/Cargo.toml` - `[workspace.package]`

Keys: `version = "0.1.0"`, `edition = "2024"`, `publish = false`, `authors = ["Michael
Alexander <beefsack@gmail.com>"]`. Do **not** add `license`/`repository` (nothing
publishes) or `rust-version` (pinned in `rust-toolchain.toml`). Members replace those
four `[package]` fields with `field.workspace = true`; `name` stays per-crate.
`bot`/`web`/`operator` gain `authors` by inheritance - intended.

### 3b. `rust/Cargo.toml` - `[workspace.dependencies]`

Hoist exactly the keys used by 2+ member manifests. Verified live list: `serde`,
`tokio`, `rand`, `serde_json`, `thiserror`, `tracing`, `time`, `tracing-subscriber`,
`sentry`, `sentry-tracing`, `reqwest`, `axum`, `anyhow`, `uuid`, `sqlx`, `warp`,
`serde_yaml`, `rustls`, `lazy_static`, `hex`, `getrandom`, `futures-util`, `async-nats`,
`aes-gcm`.

- Root version = the most precise spelling in use after step 0; never leave a bare-major
  (`"1"`) spelling. Root entry carries only features **every** consumer needs (rider 6).
- Members write `dep.workspace = true` plus their own `features`, `optional`,
  `default-features`; per-crate features are **additive**, you cannot subtract from the
  root. `[dev-dependencies]` likewise. `path` (`brdgme_*`) and 1-consumer deps stay put.
- Where a key differs by version across crates (`sqlx` 0.8 vs 0.9, `getrandom` 0.3 vs
  0.4) and step 0 does not converge them, leave it **out** of the root table (WP-66).

### 3c. `rust/Cargo.toml` - `[workspace.lints]`

Add `[workspace.lints.rust]` + `[workspace.lints.clippy]`, plus
`[lints] workspace = true` in every member. **Warning-clean, non-behavioural baseline
only** - no `unwrap_used`, `todo`, `expect_used`. The point is the plumbing so later
packages tighten in one root edit; if a lint fires, drop it rather than editing
`rust/` source.

### 3d. `rust/web/Cargo.toml` - `dp F9` ownership (do NOT do it twice)

`dp F9` (pin `tower-http`, `gloo-net`, `gloo-timers`) is a row in
`planning/checklists/T3-B8-workspace-hygiene-red7-docs.md`, GATED on D-19 and deferred
until WP-64 lands so the value is written once. All three are `web`-only, so they do
**not** enter `[workspace.dependencies]`; they stay in `rust/web/Cargo.toml` at whatever
step 0 produces. **WP-64 does not perform the downgrade** - it records, via
`cargo tree -d`, whether they still duplicate.

**Genuine tension - escalate, do not silently pick.** F9 wants a pin *back* to
tower-http 0.6 / gloo-net 0.6 / gloo-timers 0.3 to dedupe; the standing strategy says
stay on latest. If step 0 moves leptos/reqwest/kube-client onto the newer lines the
duplication dissolves and F9 is moot - close it. Otherwise ask Michael whether WASM
bundle size beats latest-first.

## 4. Non-goals

- WP-65-or-later, not here: sqlx 0.8/0.9 unification (WP-66), the duplicated
  `[profile.wasm-release]` in `rust/web/Cargo.toml`, members sorting, no-op profiles,
  `deny.toml`, `lazy_static` -> `LazyLock`, sentry `default-features = false`,
  `term_size`, game-crate `tokio = ["full"]` trimming.
- No source edits under `rust/**/src/`, and no lint that would require one.

## 5. Regression test cases

- **Never run a bare workspace-wide `cargo build`/`test`** (AGENTS.md "Resource
  constraints"). Sweep with `cargo check -p <crate>`: at minimum `-p web --features ssr`
  and `--features hydrate` (that feature split is likeliest to break on a feature
  hoist), `-p bot`, `-p operator`, `-p brdgme_cmd`, `-p brdgme_fuzz`, `-p tic-tac-toe-2`.
  Then `cargo metadata` shows all 40 members at `0.1.0` / `2024` / publish-disabled,
  with `authors` non-empty for **all 40**.
- `cargo tree -d` before and after: **no new** duplicate (one means a bad hoist).
- Behaviour unchanged, named: `cargo leptos build --release` still resolves
  `lib-profile-release = "wasm-release"` against the root profile and emits hashed
  `pkg/` assets; a game crate's four bins still build, `*_cli` plays a turn.
- Zero new warnings from the CI clippy split in `scripts/rust-ci-commands.sh`
  (`--workspace --exclude web`, then `-p web --features ssr`, both `-D warnings`); if
  either fires the lint set is too strict (3c). Final gate: `scripts/rust-test.sh`
  passes - the only sanctioned full run.

## 6. Riders

| # | Item | Source |
|---|------|--------|
| 1 | No bare-major spellings (`"1"`, `"2"`, `"1.0"`) in the root table | dp F1 |
| 2 | `bot`/`web`/`operator` gain `authors` by inheritance - do not re-add locally | dp F2 |
| 3 | No `license`/`repository`/`rust-version` in `[workspace.package]` | dp F2 |
| 4 | `[workspace.lints]` baseline warning-clean; drop any lint that fires | dp F3 |
| 5 | `dp F9` outcome recorded in the T3-B8 checklist row (see 3d) | dp F9 |
| 6 | serde `derive` is in 35 of 36 declarations: either omit it root-side and let `tools/fuzz` add it, or include it and confirm fuzz is unharmed. Pick one, say which in the PR | dp F1 |
