# WP-71: port the game-service HTTP layer from warp 0.4 to axum 0.8

**Findings:** dp F16, ls F25 (both minor; ls F25 CONFIRMED in
`findings/verification/lib-support.md`). **Decision:** D-22 - port now.

**Landing order:** **after WP-06 (HARD gate)** and after WP-64
(`landing-order.md` 8.3). WP-06 fixes the production panic *within warp*; doing
WP-71 first means writing that fix twice. No conflict with WP-68 (`repl.rs`).
WP-69 lands last.

> **Read every named file/function before editing. No line numbers are cited on
> purpose; the tree is under concurrent edit. If a file does not match what this
> spec describes, STOP and report rather than improvising.**

## 0. Step 0 - upgrade to latest FIRST (binding, do not skip)

Michael's standing strategy is to stay as close to latest as possible so deps
never go stale: the first step for any dependency problem is **"upgrade all
dependencies to latest and see where we stand."** **Here it will not help -
warp-vs-axum is a framework choice, not a version skew.** WP-64 runs the upgrade
once for the cluster; confirm it happened and proceed.

## 1. Problem

**dp F16 / ls F25** - `brdgme_cmd`'s default-on `http-server` feature uses
`warp 0.4.3` while `web`, `bot` and `operator` use `axum 0.8.9`: two HTTP server
frameworks in one workspace, warp compiled into all 27-28 game binaries.

## 2. Why it's wrong - honestly scoped

Both are **correct as written, including dp F16's own caveat, which you must not
oversell**: warp 0.4 already shares hyper 1 / http 1.x with axum, and `axum
0.8.9` is already in the lock (`web`, `lib/game_client` dev-deps), so this is a
**dedupe of one framework layer, not removal of a second HTTP stack**. The real
argument is maintenance: warp is single-maintainer with a slower cadence.

## 3. Live state of `rust/lib/cmd/src/http.rs` - CHECK FIRST

**As of 2026-07-26 WP-06 Task 1 has ALREADY LANDED.** Verified live: a private
`route::<G>()` returning a warp `Filter`, with
`content_length_limit(MAX_CONTENT_LENGTH)` (16 MiB const) before
`warp::body::json()`; `g.request(&req).unwrap_or_else(|e| Response::SystemError
{ message: e.to_string() })`; no `impl Reject for RequestError`; and three tests
in `#[cfg(test)] mod tests` -
`malformed_game_json_returns_system_error_not_panic`,
`valid_request_still_served`, `oversized_content_length_is_rejected`.
`lib/cmd/src/lib.rs` gates `pub mod http` on `feature = "http-server"`.
**Re-read and confirm.** If WP-06 has been reverted or the file differs, STOP
and report - do not port an unfixed surface, do not re-derive WP-06's fix here.

## 4. Required end state

### 4a. `rust/lib/cmd/src/http.rs`

- `route::<G>()` stays private and stays the testable seam, now returning an
  `axum::Router`.
- **The router must match ANY path.** warp's `warp::post()` matched POST on every
  path, and game-service URIs come from the `game_versions.uri` database column -
  operator-configured, unknown at compile time. Use `Router::new().fallback(..)`
  with a method guard, **not** `.route("/", post(..))`; getting this wrong
  silently breaks every deployed game version whose URI carries a path.
- Handler extracts `HeaderMap` and `Json<Request>`, keeps the same header-pair
  collection and the same `unwrap_or_else(.. => Response::SystemError {
  message })`, returns `Json(response)`. **Non-negotiable** - it is ls F19.
- Body cap: `DefaultBodyLimit::max(MAX_CONTENT_LENGTH as usize)` on the router,
  const unchanged at 16 MiB. **One accepted behaviour change:** warp's
  `content_length_limit` *required* a `Content-Length` header (411 without);
  axum's does not, so unsized bodies are accepted again and rejected at 413 only
  over the cap. That relaxes a WP-06 side effect, not the cap - record it.
- **Sentry: keep the hand-rolled transaction code** - `continue_from_headers` ->
  `start_transaction` -> `configure_scope(|s| s.set_span(..))` ->
  `transaction.finish()` on every path, changing only how headers are obtained.
  *Justification:* adopting `sentry_tower::{NewSentryLayer, SentryHttpLayer}` as
  `rust/web/src/router.rs` does would add a dependency to a crate compiled into
  28 binaries and rename transactions by route path - a single catch-all here -
  losing the explicit `"game.request"`/`"http.server"` naming, for no gain.
  Distributed tracing must survive: `lib/game_client::send_with_retry` injects
  `sentry-trace`/`baggage` via `span.iter_headers()`; keep reading them.
- `serve::<G>`'s body: `env_logger::init()` and the `sentry::init` guard block
  unchanged; replace `warp::serve(..).bind(..).graceful(..).run()` with
  `tokio::net::TcpListener::bind` + `axum::serve(listener, route::<G>())
  .with_graceful_shutdown(..)`, SIGTERM future verbatim.
  `rust/web/src/main.rs` has a working example of this shape.
- **HARD CONSTRAINT: `pub async fn serve<G: ..>(addr: impl Into<SocketAddr>)`
  must not change** - name, generic bounds or signature. All 27-28 game crates
  have a `*_http` bin calling `http::serve::<Game>(addr)`; confirm
  `rust/game/tic-tac-toe-2/src/bin/tic_tac_toe_2_http.rs` compiles untouched.

### 4b. `rust/lib/cmd/Cargo.toml`

Verified live: `default = ["http-server"]`, `http-server = ["warp", "tokio",
"sentry"]`; `warp = { version = "0.4.3", features = ["server"], optional = true
}`, `tokio = { version = "1", features = ["signal"], optional = true }`,
`sentry = { version = "0.48", optional = true }`; `env_logger` **not** gated
(rider 4). Changes: remove the `warp` dependency **and** the `warp`
dev-dependency; add `axum = { version = "0.8.9", optional = true }` (no
`tower-http`, no direct `hyper` - axum 0.8 provides `DefaultBodyLimit` and
`axum::serve`); give optional `tokio` what `axum::serve` needs (`net` plus a
runtime driver) alongside `signal`; `http-server = ["axum", "tokio", "sentry"]`;
add dev-dep `tower = { version = "0.5", features = ["util"] }` for
`ServiceExt::oneshot`, existing `tokio` dev-dep stays.

### 4c. Tests - WP-06's three, ported

Same names, same assertions.
`warp::test::request().method("POST").json(&req).reply(&route::<TestGame>())`
becomes an `http::Request` with an `axum::body::Body` driven through
`route::<TestGame>().oneshot(req)`; read the body with `axum::body::to_bytes`.
Expected statuses unchanged: 200 / 200 / 413. Do not weaken an assertion to make
a test pass - if the oversize test cannot produce 413, STOP and report.

## 5. Non-goals

- **Re-deriving WP-06's fix.** If WP-06 has not landed, stop.
- `repl.rs`/`term_size` (WP-68); sentry feature trimming (WP-67 - leave the
  `sentry` entry's features alone).
- Consolidating the 28 `*_http` bins - WP-73, where D-20 chose a generic bin
  crate **explicitly not a macro**. Do not reach for a macro here.
- Adopting `sentry-tower` (ruled out in 4a); any change to the wire protocol,
  the `Request`/`Response` types, or the routing contract `lib/game_client` sees.

## 6. Regression test cases

- **Never run a bare workspace-wide `cargo build`/`test`/`clippy`** (AGENTS.md
  "Resource constraints"). Use `cargo check -p brdgme_cmd`,
  `cargo check -p brdgme_cmd --no-default-features` (gating still clean),
  `cargo check -p tic-tac-toe-2` (bin signature held), `cargo test -p
  brdgme_cmd` (the three ported tests pass).
- `warp` absent from `rust/Cargo.lock`; `cargo tree -d` shows no new duplicate.
- **End-to-end against a running `tic_tac_toe_2_http`**, driven by `curl`:
  (1) POST each `Request` variant - `New`, `Status`, `Play`, `PubRender`,
  `PlayerRender`, `PlayerCounts` - and diff each JSON response against the same
  POST to a pre-port build; byte-identical. (2) POST `Status` with a malformed
  `game` string: HTTP **200** with `{"SystemError":{"message":"failed to parse
  request: .."}}`, connection **not** dropped - the ls F19 guarantee. (3) POST
  to a **non-root path** (e.g. `/game`) - still served. (4) POST with
  `sentry-trace` + `baggage` set; confirm in Sentry the child transaction
  attaches to the parent trace, not a new one.
- CI clippy split: `cargo clippy --workspace --exclude web --all-targets -- -D
  warnings`, then `cargo clippy -p web --all-targets --features ssr -- -D
  warnings`. Final gate: `scripts/rust-test.sh`.

## 7. Riders

| # | Item | Source |
|---|------|--------|
| 1 | `serve::<G>`'s public signature byte-identical before/after | dp F16 blast radius |
| 2 | Catch-all routing preserved (DB-supplied URIs may carry paths) | `game_versions.uri` |
| 3 | The 411-on-missing-Content-Length relaxation recorded in the PR | WP-06 Task 1 |
| 4 | `env_logger` ungated but used only by the gated `serve` - note for WP-65, **do not fix here** | live manifest |
| 5 | Pre-existing: `configure_scope` sets the span on a shared hub, no per-request `Hub::run`; same under warp, not a regression - record, do not fix | live `http.rs` |
| 6 | PR text says "one framework layer", not "a second HTTP stack" | dp F16 |
