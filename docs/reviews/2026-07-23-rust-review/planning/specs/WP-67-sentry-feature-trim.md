# WP-67: sentry feature trim

**Finding:** dp F12 (major). **Decision:** D-18 - trim to explicit features,
verified with `cargo tree`, under a hard standing constraint: **no Sentry
functionality may be lost.**

**Landing order:** after WP-64 (`sentry`/`sentry-tracing` are in its hoist
list, so this becomes a one-line root edit). Independent of WP-66 - either
order. WP-69 lands last.

> **Read every named file/table/function before editing. No line numbers are
> cited on purpose; the tree is under concurrent edit. If a file does not match
> what this spec describes, STOP and report rather than improvising.**

## 0. Step 0 - upgrade to latest FIRST (binding, do not skip)

Michael's standing strategy is to stay as close to latest as possible so deps
never go stale. First step: **"upgrade all dependencies to latest and see where
we stand"**. Bump `sentry`, `sentry-tracing`, `sentry-tower` and re-resolve
before touching features. **If the upgrade already removes `sentry-actix`,
`actix-*` and `ureq` from the graph, this spec collapses to section 3a (spell
the feature list explicitly, change nothing else) and 3b is dropped.** Feature
names below are 0.48.5's; re-read the `[features]` table if Step 0 moves major.

## 1. Problem

**dp F12** - all four sentry declarations (`rust/bot/Cargo.toml`,
`rust/web/Cargo.toml`, `rust/lib/cmd/Cargo.toml`,
`rust/lib/game_client/Cargo.toml`) are `sentry = "0.48"` with default features.
The finding says this drags `sentry-actix` (+8 `actix-*` packages) and `ureq 3`
into every server build and all 28 game binaries.

## 2. Why it's wrong - and where the finding is itself unreliable

- Confirmed live: `bot` `sentry = "0.48"`; `web`, `lib/cmd`, `lib/game_client`
  all `{ version = "0.48", optional = true }`. **No crate sets
  `default-features = false` today.** All four ask for an identical feature
  set, so WP-64's hoist is clean. `sentry-tower` (web only, `features =
  ["http"]`) and `sentry-tracing` (web + bot) are separate crates, unaffected.
- **The finding's mechanism is questionable and must be re-verified before any
  removal.** In sentry 0.48.5 `default = ["backtrace", "contexts",
  "debug-images", "panic", "transport", "release-health"]` and `transport =
  ["reqwest", "native-tls"]`. **`actix` and `ureq` are NOT default features.**
  Yet `rust/Cargo.lock` lists both `sentry-actix` and `ureq` under the `sentry`
  package while other unused optionals of the same crate (`curl`, `sentry-log`,
  `sentry-slog`, `sentry-anyhow`) are absent. Reconcile that before deleting.
- **First implementation action is measurement, not editing.** Run and record
  in the PR: `cargo tree -p bot -i actix-web`, `cargo tree -p bot -i ureq`,
  `cargo tree -p web --features ssr -i actix-web` / `-i ureq`, and
  `cargo tree -p tic-tac-toe-2 -i actix-web` (game binaries reach sentry via
  `brdgme_cmd`'s `http-server` feature). **If those come back empty, dp F12's
  build-bloat claim is false**, the lock entries are resolution artefacts, the
  finding is downgraded from major, and the package reduces to 3a.

## 3. Required end state

### 3a. Explicit feature spelling (always done)

`sentry` in `[workspace.dependencies]` becomes `default-features = false` with
the in-use features spelled out. **Enumerate before removing:** grep
`sentry::`, `sentry_tracing`, `sentry_tower` across `rust/` and map each call
to its feature. Verified live call sites:

| Call site | API | Feature |
|---|---|---|
| `web/src/main.rs::init_sentry`, `bot/src/main.rs::main`, `lib/cmd/src/http.rs::serve` | `sentry::init` + `ClientOptions { release, send_default_pii, traces_sample_rate }` | core |
| `lib/cmd/src/http.rs`, `bot/src/main.rs` | `TransactionContext::continue_from_headers`, `start_transaction`, `scope.set_span` | core (performance) |
| `web/src/router.rs` | `configure_scope(... set_transaction)`, `sentry_tower::{NewSentryLayer, SentryHttpLayer}` | core + separate `sentry-tower` crate |
| `web/src/game/mod.rs`, `lib/game_client/src/lib.rs` | `configure_scope` | core |
| `web/src/main.rs`, `bot/src/main.rs` | `sentry_tracing::layer()` | separate `sentry-tracing` crate |

Resulting list - exactly today's defaults, spelled out: `["backtrace",
"contexts", "debug-images", "panic", "release-health", "reqwest",
"native-tls"]` (`reqwest` + `native-tls` == the `transport` default). Keeping
`debug-images` and `release-health` is deliberate: dropping either loses
functionality (native-frame symbolication; release/session health), which D-18
forbids. The findings' suggested list omitted `release-health` - it is wrong.
**`native-tls` is non-negotiable**: `rust/web/Cargo.toml` records it as
deliberate, keeping sentry out of the rustls crypto-provider selection
(`docs/CODING.md`). Do not substitute `rustls`, `rustls-no-provider`, `ureq`.

### 3b. Confirmation

3a's list already excludes `actix` and `ureq`; re-run the `cargo tree -i`
commands and confirm they are empty. If still non-empty the source is something
other than sentry's defaults: **STOP and report**, do not start deleting.

## 4. Non-goals

- sqlx (WP-66), `deny.toml` (WP-69), `warp` (WP-71), `term_size` (WP-68); the
  svix http-0.2 duplicate (re-audit only, after this lands).
- Changing `ClientOptions` values - `send_default_pii: false` and
  `traces_sample_rate: 0.1` stay exactly as they are in all three init sites.
- Adding sentry's own `tracing`/`tower`/`tower-http` features; web and bot
  depend on `sentry-tracing`/`sentry-tower` directly and keep doing so.

## 5. Regression test cases

- **Never run a bare workspace-wide `cargo build`/`test`/`clippy`** (AGENTS.md
  "Resource constraints"). Use `cargo check -p bot`,
  `-p web --features ssr`, `-p operator`,
  `-p brdgme_cmd --features http-server`,
  `-p brdgme_game_client --features sentry`, `-p tic-tac-toe-2`.
- `cargo tree -d` before/after; no new duplicate.
- **End-to-end check, not just `cargo tree`.** Point a dev build at a real DSN
  (`SENTRY_DSN_SERVER` + `SENTRY_RELEASE`) and confirm, for **each of `web`
  (ssr), `bot`, and one game binary (`tic-tac-toe-2_http`)**:
  1. a deliberate `panic!` produces an event (panic integration);
  2. that event carries a resolved **backtrace** with frames;
  3. `tracing::error!` produces an event and earlier `tracing` events appear as
     breadcrumbs (`sentry_tracing::layer()`);
  4. the event carries **release**, **environment**, **server name** and OS/
     runtime contexts (contexts + debug-images);
  5. for `web`, a request produces a transaction named by
     `configure_scope(... set_transaction)` and the `sentry_tower` layers still
     apply;
  6. for `bot` and the game binary, a request carrying `sentry-trace`/`baggage`
     headers continues the **distributed trace** via `continue_from_headers` -
     the child transaction attaches to the parent, not a new trace.
  Compare each field against an event from an untrimmed build. Any field that
  disappears fails the package - restore the feature.
- CI clippy split: `cargo clippy --workspace --exclude web --all-targets -- -D
  warnings`, then `cargo clippy -p web --all-targets --features ssr -- -D
  warnings`. Final gate:
  `/home/beefsack/Development/brdgme/scripts/rust-test.sh`.

## 6. Riders

| # | Item | Source |
|---|------|--------|
| 1 | `cargo tree -i actix-web` / `-i ureq` output recorded **before** any edit | dp F12 |
| 2 | If actix/ureq are lock-only artefacts, dp F12 is downgraded from major and that is written into the finding | dp F12 |
| 3 | `native-tls` retained; no `rustls`/`rustls-no-provider`/`ureq` feature added | D-18 |
| 4 | `debug-images` and `release-health` retained | D-18 |
| 5 | Single hoisted `sentry` entry; all four members use `sentry.workspace = true` (three keeping `optional = true`) | WP-64 |
