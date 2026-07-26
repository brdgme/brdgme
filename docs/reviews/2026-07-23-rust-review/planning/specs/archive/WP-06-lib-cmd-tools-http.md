# WP-06: lib cmd tools and http

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Eliminate the one production-runtime defect in `brdgme_cmd` — the warp handler `.unwrap()` that panics every game service on a malformed request (ls F19) — and land the dev-tool robustness and dead-code cleanups in the same crate: render-path panic consistency in the requester and CLI (ls F23), REPL stale renders after `:undo`/`:load` (ls F20), REPL hot-spin on stdin EOF (ls F22), first-`:undo` silent reset (ls F30), the `comparison_to_empty` nit (ls F26), HTTP body size cap (ls F28), child exit-status reporting in the local requester (ls F29), the redundant `#[serde(default)]` (ls F27), `bot_cli` dead code (ls F21), and two `rand_bot` nits (ls F44, ls F45).

**Architecture — how `brdgme_cmd` works (read this before editing):**

- Crate `rust/lib/cmd` (package name `brdgme_cmd`, lib layout in `src/lib.rs`): `api.rs` (the `Request`/`Response` wire enums shared by every transport), `requester/` (`Requester` trait; `gamer.rs` = in-process implementation over a `Gamer` type; `local.rs` = spawn-a-child-binary implementation; `error.rs` = `RequestError`), `cli.rs` (stdin/stdout JSON transport used by all 27 game `*_cli` bins), `http.rs` (warp HTTP transport used by all game service bins — behind the default-on `http-server` feature), `repl.rs` (interactive dev REPL used by `tools/repl` and the game `*_repl` bins), `bot_cli.rs` (bot request struct + a dead CLI fn), `test_support.rs` (contract harness, `test-support` feature).
- **The HTTP path is production**: every deployed game service is a tiny bin calling `http::serve::<Game>(addr)`. The handler (http.rs:36-57) is a sync `warp::Filter::map` closure: deserialize `Request`, run it through `GameRequester`, reply JSON. `GameRequester::request` returns `Err(RequestError::Parse)` whenever the embedded `game` state string is not valid JSON (gamer.rs:28,37,41,45) — attacker/caller-influenced input — and http.rs:54 `.unwrap()`s it, panicking the connection task. The wire protocol already has an in-band error channel: `Response::SystemError { message }`, which `gamer.rs` itself uses for state-encode failures and which the web side (`lib/game_client` / `web`) already handles. `impl Reject for RequestError {}` at http.rs:17 is never used (abandoned wiring).
- `cargo test -p brdgme_cmd` runs with default features, so `http.rs` and its `#[cfg(test)]` tests compile in a normal per-crate test run.
- There is currently **no `Gamer` implementation available inside `brdgme_cmd` tests** (game crates depend on `brdgme_cmd`, so dev-depending on one would be a cycle). Task 1 adds a minimal `#[cfg(test)] mod test_game` used by the http and requester tests.
- `repl.rs` is interactive by construction (`stdin()`/`stdout()` hard-wired in `prompt`/`output_*`); its fixes are verified by build + clippy + an optional manual smoke, not unit tests.
- `bot_cli.rs`: workspace grep confirms the only consumer is `lib/rand_bot`, which uses only `bot_cli::Request` (rand_bot lib.rs:107). `bot_cli::cli` (four `.unwrap()`s — bot_cli.rs:29,30,41,43, per verification correction) and `pub type Response` are dead.
- **Serialization compatibility:** `api::Request`/`api::Response` are the deployed wire format between `web` ↔ game services and the on-disk format of `repl` save files. No change in this package may alter how any existing message serializes. Every change below is control flow, error mapping, or dead-code removal; the ls F27 attribute removal is a no-op for both serialize and deserialize (serde defaults missing `Option` to `None` regardless).

**Tech Stack:** Rust 1.97.0 (edition 2024), workspace at `/home/beefsack/Development/brdgme/rust`. Crates touched: `brdgme_cmd` (main), `brdgme_rand_bot` (two nits). warp 0.4.3 (`server` feature; `warp::test` is available unconditionally — verified in the vendored 0.4.3 source, `src/lib.rs:103`). New dev-dependency in Task 1: `tokio` with `macros` + `rt` (for `#[tokio::test]`; tokio 1.x is already an optional dependency, so this unifies).

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p brdgme_cmd`, `cargo test -p brdgme_rand_bot`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- docs/CODING.md "No panicking code in runtime paths" applies to `http.rs`, `gamer.rs`, `cli.rs`, `local.rs`. The REPL is exempted by explicit finding disposition (ls F23: "leave the repl's panics if it is strictly a dev toy") — repl panics stay, but get real messages.
- The 27 game crates' `*_cli`/`*_repl`/service bins compile against `brdgme_cmd`'s public API. Nothing in this package changes a signature those bins call (`cli::cli`, `repl`, `http::serve`, `requester::gamer::new`). The only public-signature change is `requester::gamer::renders` → `Result` (Task 2), which has zero external callers (workspace grep verified).
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests need the containers the script provides; pre-existing, backlog #40).

**Non-Goals (owned elsewhere — do NOT touch):**

- ls F24 (term_size → terminal_size, repl.rs:186 + Cargo.toml:16) — WP-68. Leave the `term_size::dimensions()` call and the dependency alone.
- ls F25 / dp F16 (warp → axum consolidation) — WP-71, blocked on D-22. This package fixes the handler *within warp*; do not change frameworks. WP-71's spec must re-apply the F19/F28 semantics (SystemError mapping + body cap) when it ports — noted for its author.
- ls F31–F37 (`lib/game_client`), ls F40–F43 (`rand_bot` lib.rs timeout/join/features/panics) — WP-07. Task 4 touches rand_bot's `main.rs` and one comment block in `lib.rs` only; do not fix the lib.rs:50/84/107 unwraps (ls F43) or the join separator here.
- Player-index bounds validation at the requester boundary (`game.player_state(player)` with an out-of-range `player` in `handle_player_render`, findings e F18/e F36) — WP-09, blocked on D-36. Task 2 wraps only the *serialization* of render states; the `player_state(p)` call itself stays as-is.
- The markup-parse unwraps in `repl.rs:177,219` operate on game-produced log/render content; the underlying markup parser robustness is WP-02. Per ls F23's disposition these repl unwraps stay.
- lib/markup, lib/color, lib/cost findings (ls F1–F18, F38, F39) — WP-01/02/05/17.

**Snapshot drift:** None. `diff` of every touched file (`lib/cmd/src/{http,repl,bot_cli,cli,api,lib}.rs`, `lib/cmd/src/requester/{gamer,local,error,mod}.rs`, `lib/cmd/Cargo.toml`, `lib/rand_bot/src/{main,lib}.rs`, `lib/rand_bot/Cargo.toml`) against `/home/beefsack/Development/brdgme-review-snapshot/rust` (commit f8763a5) is empty — verified 2026-07-25. All line numbers below are live-file line numbers and match the findings' citations.

**Re-derivation of ls F19 (the urgent item) — verified against live source:** `Request::Status`/`Play`/`PubRender`/`PlayerRender` all carry the game state as a JSON *string* field; `GameRequester::request` runs `serde_json::from_str(game)?` on it (gamer.rs:28,37,41,45), so `Err(RequestError::Parse{..})` is reachable by ANY caller sending a syntactically invalid `game` — no game-crate bug required. http.rs:54 `g.request(&req).unwrap()` then panics inside the warp `map` closure: the client sees a dropped connection/500 instead of the protocol's `SystemError` JSON, `transaction.finish()` (http.rs:55) is skipped, and the panic bypasses the sentry transaction. The finding's recommendation (`unwrap_or_else` → `Response::SystemError`) is CONFIRMED and is exactly what the sibling transport `cli.rs` already does for its own parse failures — the wire contract explicitly supports it. One refinement: `RequestError::Parse`'s `Display` is `"failed to parse request"` with the serde detail only in the unrendered `source`; the message is made useful by adding `{source}` to the thiserror attribute (error.rs:7). Nothing asserts on that string (workspace grep).

---

### Task 1: never panic the production HTTP handler; cap the body (ls F19 MAJOR + ls F28 nit)

**Fix (re-derived):** extract the filter chain from `serve` into a private `route::<G>()` so it is unit-testable, map requester errors into `Response::SystemError`, delete the dead `impl Reject`, and prepend `warp::body::content_length_limit`. Include the serde detail in `RequestError::Parse`'s message.

Behavioral changes at the HTTP boundary, before → after:

- Malformed `game` string in a request: connection dropped by panic → HTTP 200 with `{"SystemError":{"message":"failed to parse request: <serde detail>"}}` (the shape `web`/`game_client` already consume; `gamer.rs` already returns `SystemError` in-band for other failures).
- Request without a `Content-Length` header: previously accepted → now 411 (warp's `content_length_limit` requires the header). Safe: the only HTTP callers are `lib/game_client` (reqwest with a buffered `.json()`/`.body()` — always sends `Content-Length`) and dev `curl` (same).
- Request with `Content-Length` > 16 MiB: previously read fully → now 413 without reading the body. 16 MiB is ~30x the largest observed serialized game state; generous on purpose, this is DoS hygiene, not a tight bound (ls F28: "in-cluster and low risk").
- Sentry: the transaction now always reaches `transaction.finish()`.

**Files:**
- Modify: `rust/lib/cmd/src/http.rs` (whole handler section), `rust/lib/cmd/src/requester/error.rs` (one attribute), `rust/lib/cmd/src/lib.rs` (register test module), `rust/lib/cmd/Cargo.toml` (dev-deps)
- Create: `rust/lib/cmd/src/test_game.rs` (`#[cfg(test)]`-only minimal `Gamer` impls, shared with Task 2's tests)

**Interfaces:** `pub async fn serve<G>(addr)` unchanged. New private `fn route<G>()` in http.rs. `test_game` is `#[cfg(test)]` and never ships.

**Steps:**

- [ ] Add the test scaffolding. Create `rust/lib/cmd/src/test_game.rs`:

```rust
//! Minimal in-crate `Gamer` implementations for transport tests. Game crates
//! depend on `brdgme_cmd`, so tests here cannot use a real game (dependency
//! cycle); these stand-ins cover the happy path (`TestGame`) and the
//! render-serialization failure path (`BrokenRenderGame`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::{CommandResponse, Gamer, Log, Renderer, Status};
use brdgme_markup::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGame {
    pub players: usize,
    pub plays: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TestState;

impl Renderer for TestState {
    fn render(&self) -> Vec<Node> {
        vec![Node::text("test")]
    }
}

impl Gamer for TestGame {
    type PubState = TestState;
    type PlayerState = TestState;

    fn start(players: usize, _seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !Self::player_counts().contains(&players) {
            return Err(GameError::PlayerCount {
                min: 1,
                max: 4,
                given: players,
            });
        }
        Ok((TestGame { players, plays: 0 }, vec![]))
    }

    fn pub_state(&self) -> TestState {
        TestState
    }

    fn player_state(&self, _player: usize) -> TestState {
        TestState
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        _players: &[String],
    ) -> Result<CommandResponse, GameError> {
        if player != 0 {
            return Err(GameError::NotYourTurn);
        }
        match input.trim().strip_prefix("play") {
            Some(rest) => {
                self.plays += 1;
                Ok(CommandResponse {
                    logs: vec![],
                    can_undo: false,
                    remaining_input: rest.to_string(),
                })
            }
            None => Err(GameError::invalid_input("expected 'play'")),
        }
    }

    fn status(&self) -> Status {
        Status::Active {
            whose_turn: vec![0],
            eliminated: vec![],
        }
    }

    fn command_spec(&self, _player: usize) -> Option<CommandSpec> {
        None
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        vec![1, 2, 3, 4]
    }
}

/// Its pub/player state serializes to a JSON map with tuple keys, which
/// serde_json rejects at runtime ("key must be a string") - exercises the
/// render-serialization SystemError paths without a panicking game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenRenderGame {
    pub players: usize,
}

#[derive(Serialize, Deserialize)]
pub struct BrokenState {
    pub map: HashMap<(u8, u8), u8>,
}

impl Renderer for BrokenState {
    fn render(&self) -> Vec<Node> {
        vec![Node::text("broken")]
    }
}

impl Gamer for BrokenRenderGame {
    type PubState = BrokenState;
    type PlayerState = BrokenState;

    fn start(players: usize, _seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        Ok((BrokenRenderGame { players }, vec![]))
    }

    fn pub_state(&self) -> BrokenState {
        BrokenState {
            map: HashMap::from([((0, 0), 0)]),
        }
    }

    fn player_state(&self, _player: usize) -> BrokenState {
        self.pub_state()
    }

    fn command(
        &mut self,
        _player: usize,
        _input: &str,
        _players: &[String],
    ) -> Result<CommandResponse, GameError> {
        Err(GameError::invalid_input("no commands"))
    }

    fn status(&self) -> Status {
        Status::Active {
            whose_turn: vec![0],
            eliminated: vec![],
        }
    }

    fn command_spec(&self, _player: usize) -> Option<CommandSpec> {
        None
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        vec![2]
    }
}
```

  Register it in `rust/lib/cmd/src/lib.rs` after the existing module list (line 14):

```rust
#[cfg(test)]
mod test_game;
```

- [ ] Add the test runtime to `rust/lib/cmd/Cargo.toml` (new section at the end; the optional runtime `tokio` dep at line 18 is untouched):

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
```

- [ ] Write the failing tests. Append to `rust/lib/cmd/src/http.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Response;
    use crate::test_game::TestGame;

    #[tokio::test]
    async fn malformed_game_json_returns_system_error_not_panic() {
        // ls F19: a syntactically invalid `game` string must come back as an
        // in-band SystemError, not panic the connection task.
        let res = warp::test::request()
            .method("POST")
            .json(&Request::Status {
                game: "not valid json".to_string(),
            })
            .reply(&route::<TestGame>())
            .await;
        assert_eq!(200, res.status());
        match serde_json::from_slice::<Response>(res.body()).unwrap() {
            Response::SystemError { message } => assert!(
                message.contains("failed to parse request"),
                "got: {}",
                message
            ),
            r => panic!("expected SystemError, got {:?}", r),
        }
    }

    #[tokio::test]
    async fn valid_request_still_served() {
        let res = warp::test::request()
            .method("POST")
            .json(&Request::New {
                players: 2,
                seed: Some(1),
            })
            .reply(&route::<TestGame>())
            .await;
        assert_eq!(200, res.status());
        match serde_json::from_slice::<Response>(res.body()).unwrap() {
            Response::New { player_renders, .. } => assert_eq!(2, player_renders.len()),
            r => panic!("expected New, got {:?}", r),
        }
    }

    #[tokio::test]
    async fn oversized_content_length_is_rejected() {
        // ls F28: the declared length is checked before the body is read.
        // (.json() sets a correct content-length; the .header() call after it
        // replaces the value - warp::test headers use HeaderMap::insert.)
        let res = warp::test::request()
            .method("POST")
            .json(&Request::PlayerCounts)
            .header("content-length", (MAX_CONTENT_LENGTH + 1).to_string())
            .reply(&route::<TestGame>())
            .await;
        assert_eq!(413, res.status());
    }
}
```

- [ ] Run: `cargo test -p brdgme_cmd http::` — expected: compile FAILURE (`route` and `MAX_CONTENT_LENGTH` do not exist yet). This is the red step for the extraction; the panic behavior itself goes red→green in the same run below.
- [ ] Implement. Rewrite the handler section of `rust/lib/cmd/src/http.rs`:
  1. Delete line 17 (`impl Reject for RequestError {}`) and the now-unused imports `warp::reject::Reject` (line 7) and `crate::requester::error::RequestError` (line 14). Why delete rather than wire up: rejections bypass the JSON wire protocol; the protocol's own `SystemError` is the correct channel (and what every client already parses).
  2. Add `use crate::api::Response;` to the imports (line 11 already imports `Request`).
  3. Above `serve`, add the extracted route (the closure body is the existing http.rs:39-57 code with ONLY the reply line changed):

```rust
const MAX_CONTENT_LENGTH: u64 = 16 * 1024 * 1024;

fn route<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::header::headers_cloned())
        .and(warp::body::content_length_limit(MAX_CONTENT_LENGTH))
        .and(warp::body::json())
        .map(|headers: warp::http::HeaderMap, req: Request| {
            let header_pairs: Vec<(String, String)> = headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect();
            let ctx = sentry::TransactionContext::continue_from_headers(
                "game.request",
                "http.server",
                header_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            );
            let transaction = sentry::start_transaction(ctx);
            sentry::configure_scope(|scope| {
                scope.set_span(Some(transaction.clone().into()));
            });
            let mut g: GameRequester<G> = requester::gamer::new();
            let response = g.request(&req).unwrap_or_else(|e| Response::SystemError {
                message: e.to_string(),
            });
            let reply = warp::reply::json(&response);
            transaction.finish();
            reply
        });
```

     (note: the function body ends by returning that filter expression — no trailing `;` on the real return). In `serve`, replace the whole `let handler = warp::post()...` block (lines 36-57) with `let handler = route::<G>();` — the env_logger/sentry-guard init and the `warp::serve(handler)...` tail (lines 58-70) stay exactly as they are.
  4. In `rust/lib/cmd/src/requester/error.rs` line 7, change `#[error("failed to parse request")]` to `#[error("failed to parse request: {source}")]`. Why: the SystemError message must tell the operator *what* failed to parse; thiserror does not render `source` unless asked, and no code or test matches the old string.
- [ ] Run: `cargo test -p brdgme_cmd` — all 3 new http tests PASS; the pre-existing api.rs datetime tests PASS.
- [ ] `cargo clippy -p brdgme_cmd --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Sanity-check a downstream service bin still compiles against the crate: `cargo check -p tic-tac-toe-2` (its `src/bin/` includes the http service bin; `serve`'s signature is unchanged so this must pass untouched).
- [ ] Commit: `git add rust/lib/cmd/src/http.rs rust/lib/cmd/src/requester/error.rs rust/lib/cmd/src/lib.rs rust/lib/cmd/src/test_game.rs rust/lib/cmd/Cargo.toml` ; message: `fix(cmd): return SystemError instead of panicking in the HTTP handler, cap body size (ls F19 ls F28, WP-06)`

---

### Task 2: consistent error paths in the requester and CLI transports (ls F23, minor)

**Problem (restated, all sites verified live):** the crate converts some runtime failures into `Response::SystemError` and `.unwrap()`s identical ones a few lines away:

- `gamer.rs` routes *game-state* serialization through `GameResponseError` (`from_gamer`) but `.unwrap()`s *render-state* serialization at gamer.rs:67,74 (inside `renders`, called by New/Status/Play handling) and gamer.rs:164,177 (`handle_pub_render`/`handle_player_render`). A `PubState`/`PlayerState` that fails `serde_json::to_string` (e.g. a map with non-string keys) panics the process — over HTTP that was the F19 class again, one layer down.
- `cli.rs:14-18` converts a request-envelope parse failure into `SystemError` but `.unwrap()`s the requester error and the output writes.

**Fix (re-derived, confirms the finding's recommendation):**

1. `renders` returns `Result<(PubRender, Vec<PlayerRender>), GameResponseError>` (the existing error type already has `#[from] serde_json::Error`). Its only callers are the three handlers in the same file (workspace grep verified: no external users of `requester::gamer::renders`), which already end in `.unwrap_or_else(SystemError)` — switch their `.map(...)` to `.and_then(...)` and use `?`.
2. `handle_pub_render`/`handle_player_render` match on the serialization instead of unwrapping, reusing `GameResponseError` for a consistent message. The `game.player_state(player)` call itself is NOT guarded (WP-09 owns index validation — see Non-Goals).
3. `cli.rs`: `unwrap_or_else` the requester error into `SystemError` (mirror of its own parse branch). The two output-write panics become `expect` with real messages: a CLI process that cannot write to its own stdout has no error channel left — panicking there is the documented process-boundary exception (docs/CODING.md acceptable panics), and `serde_json::to_string` of a `Response` cannot fail (no non-string-key maps in the type).

**Files:**
- Modify: `rust/lib/cmd/src/requester/gamer.rs`, `rust/lib/cmd/src/cli.rs`
- Tests: inline `#[cfg(test)] mod tests` in each, using Task 1's `test_game`

**Steps:**

- [ ] Write the failing tests. Append to `rust/lib/cmd/src/requester/gamer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_game::{BrokenRenderGame, TestGame};

    #[test]
    fn status_render_serialization_failure_returns_system_error() {
        // ls F23: renders() unwraps serde_json inside the Status path.
        let state = serde_json::to_string(&BrokenRenderGame { players: 2 }).unwrap();
        let mut r = new::<BrokenRenderGame>();
        match r.request(&Request::Status { game: state }).unwrap() {
            Response::SystemError { message } => assert!(
                message.contains("failed to encode game state"),
                "got: {}",
                message
            ),
            resp => panic!("expected SystemError, got {:?}", resp),
        }
    }

    #[test]
    fn pub_render_serialization_failure_returns_system_error() {
        let state = serde_json::to_string(&BrokenRenderGame { players: 2 }).unwrap();
        let mut r = new::<BrokenRenderGame>();
        match r.request(&Request::PubRender { game: state }).unwrap() {
            Response::SystemError { .. } => {}
            resp => panic!("expected SystemError, got {:?}", resp),
        }
    }

    #[test]
    fn player_render_serialization_failure_returns_system_error() {
        let state = serde_json::to_string(&BrokenRenderGame { players: 2 }).unwrap();
        let mut r = new::<BrokenRenderGame>();
        match r.request(&Request::PlayerRender {
            player: 0,
            game: state,
        }) {
            Ok(Response::SystemError { .. }) => {}
            resp => panic!("expected Ok(SystemError), got {:?}", resp),
        }
    }

    #[test]
    fn status_happy_path_unchanged() {
        let state = serde_json::to_string(&TestGame::start(2, 1).unwrap().0).unwrap();
        let mut r = new::<TestGame>();
        match r.request(&Request::Status { game: state }).unwrap() {
            Response::Status { player_renders, .. } => assert_eq!(2, player_renders.len()),
            resp => panic!("expected Status, got {:?}", resp),
        }
    }
}
```

  Append to `rust/lib/cmd/src/cli.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::requester::error::RequestError;

    struct FailingRequester;

    impl Requester for FailingRequester {
        fn request(&mut self, _req: &Request) -> Result<Response, RequestError> {
            Err(RequestError::Stdin)
        }
    }

    #[test]
    fn requester_error_becomes_system_error_json() {
        // ls F23: cli converts its own parse failures but unwraps requester
        // errors.
        let input = serde_json::to_vec(&Request::PlayerCounts).unwrap();
        let mut out: Vec<u8> = vec![];
        cli(&mut FailingRequester, input.as_slice(), &mut out);
        match serde_json::from_slice::<Response>(&out).unwrap() {
            Response::SystemError { message } => assert_eq!("Failed to get stdin", message),
            r => panic!("expected SystemError, got {:?}", r),
        }
    }
}
```

- [ ] Run: `cargo test -p brdgme_cmd requester::gamer` and `cargo test -p brdgme_cmd cli::` — expected: the three `*_failure_*` tests and `requester_error_becomes_system_error_json` FAIL by PANICKING on the existing `.unwrap()`s ("key must be a string" / `RequestError::Stdin`); `status_happy_path_unchanged` passes already.
- [ ] Implement in `rust/lib/cmd/src/requester/gamer.rs`:
  1. Add `GameResponseError` to the api import (line 10): `use crate::api::{CliLog, GameResponse, GameResponseError, PlayerRender, PubRender, Request, Response};`
  2. Replace `renders` (lines 62-81) with:

```rust
pub fn renders<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    game: &G,
) -> Result<(PubRender, Vec<PlayerRender>), GameResponseError> {
    let pub_state = game.pub_state();
    let pub_render = PubRender {
        pub_state: serde_json::to_string(&pub_state)?,
        render: brdgme_markup::to_string(&pub_state.render()),
    };
    let mut player_renders: Vec<PlayerRender> = Vec::with_capacity(game.player_count());
    for p in 0..game.player_count() {
        let player_state = game.player_state(p);
        player_renders.push(PlayerRender {
            player_state: serde_json::to_string(&player_state)?,
            render: brdgme_markup::to_string(&player_state.render()),
            command_spec: game.command_spec(p),
        });
    }
    Ok((pub_render, player_renders))
}
```

  3. In `handle_new` (lines 89-102), `handle_status` (lines 111-123), and `handle_play` (lines 136-150): change `.map(|gs| { ... })` to `.and_then(|gs| { ... Ok(...) })` and the `let (public_render, player_renders) = renders(&game);` line to `let (public_render, player_renders) = renders(game)?;` (in `handle_new` the receiver is `&game`). The trailing `.unwrap_or_else(|e| Response::SystemError { message: e.to_string() })` on each is already there and stays — `GameResponseError` is the error type flowing through both `from_gamer` and `renders`, so the closures type-check unchanged otherwise.
  4. Replace `handle_pub_render` (lines 158-168) and `handle_player_render` (lines 170-182) with:

```rust
fn handle_pub_render<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    game: &G,
) -> Response {
    let pub_state = game.pub_state();
    match serde_json::to_string(&pub_state) {
        Ok(pub_state_json) => Response::PubRender {
            render: PubRender {
                pub_state: pub_state_json,
                render: brdgme_markup::to_string(&pub_state.render()),
            },
        },
        Err(e) => Response::SystemError {
            message: GameResponseError::from(e).to_string(),
        },
    }
}

fn handle_player_render<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    player: usize,
    game: &G,
) -> Response {
    let player_state = game.player_state(player);
    match serde_json::to_string(&player_state) {
        Ok(player_state_json) => Response::PlayerRender {
            render: PlayerRender {
                player_state: player_state_json,
                render: brdgme_markup::to_string(&player_state.render()),
                command_spec: game.command_spec(player),
            },
        },
        Err(e) => Response::SystemError {
            message: GameResponseError::from(e).to_string(),
        },
    }
}
```

- [ ] Implement in `rust/lib/cmd/src/cli.rs` — replace the function body (lines 6-19) with:

```rust
pub fn cli<R: Requester, I: Read, O: Write>(requester: &mut R, input: I, output: &mut O) {
    writeln!(
        output,
        "{}",
        serde_json::to_string(&match serde_json::from_reader::<_, Request>(input) {
            Err(message) => Response::SystemError {
                message: message.to_string(),
            },
            Ok(r) => requester.request(&r).unwrap_or_else(|e| Response::SystemError {
                message: e.to_string(),
            }),
        })
        .expect("failed to encode response JSON")
    )
    .expect("failed to write response to output");
}
```

- [ ] Run: `cargo test -p brdgme_cmd` — all Task 1 + Task 2 tests PASS (Task 1's http tests double as regression cover: `renders` is on the New path).
- [ ] `cargo clippy -p brdgme_cmd --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean. Also `cargo check -p tic-tac-toe-2` (confirms no game bin depended on the old `renders` signature).
- [ ] Commit: `git add rust/lib/cmd/src/requester/gamer.rs rust/lib/cmd/src/cli.rs` ; message: `fix(cmd): route render/requester failures through SystemError, not unwrap (ls F23, WP-06)`

---

### Task 3: REPL correctness — EOF, stale renders, undo seeding, nits (ls F20 + ls F22 + ls F30 + ls F26 + ls F23's repl bits)

**Problems (restated, all verified live in `rust/lib/cmd/src/repl.rs`):**

- ls F22 — `prompt` (lines 226-235) ignores `read_line`'s byte count: at EOF it returns `""` forever. Mid-game that means an infinite hot loop firing empty `Play` commands (each printing a `UserError`); during name entry it silently proceeds with the players entered so far. `read_line` errors are `.unwrap()`ed (line 233).
- ls F20 — `:load` (112-115) and `:undo` (116-128) replace `game` but not `public_render`/`player_renders`; the next iteration renders the pre-undo board (lines 91-98) while `Play` is built from the restored `game.state` (line 135) — display and actual state diverge until the next successful play.
- ls F30 — `undo_stack` is seeded with the initial game (line 59), so the first `:undo` before any play silently "resets" (to the state you are already in, then — pre-F20-fix — with a stale render) instead of reporting nothing to undo. The push-on-play at line 152 is already correct.
- ls F26 — `remaining_input.trim() != ""` (line 147), clippy `comparison_to_empty`.
- ls F23 (repl portion) — `panic!("wrong reponse")` (line 55, typo + content-free) and `panic!("unexpected response")` (line 167). Panics stay (dev toy, per disposition); messages get content.

**Fix (re-derived):** `prompt` returns `Option<String>` (`None` = EOF or unreadable stdin), both call sites treat `None` as quit; a `refresh_renders` helper re-requests `Request::Status` after `:undo`/`:load` and adopts its renders (the finding's first recommended option — the alternative, storing renders on the undo stack, would not fix `:load` and would grow the stack entries for no benefit); the undo stack starts empty.

**No unit tests** — the REPL reads `stdin()`/writes `stdout()` directly and every path is interactive; refactoring it for injectable I/O is out of scope for this package. Verification is compile + clippy (which enforces the F26 fix via `comparison_to_empty`) + an optional manual smoke below.

**Files:**
- Modify: `rust/lib/cmd/src/repl.rs` only

**Steps:**

- [ ] In the imports (line 11), extend the api use to `use crate::api::{CliLog, GameResponse, PlayerRender, PubRender, Request, Response};`.
- [ ] Replace `prompt` (lines 226-235) with:

```rust
/// Prompts and reads one trimmed line. `None` means stdin is exhausted or
/// unreadable (EOF from a pipe, parent process gone) - callers must treat it
/// as quit; pre-fix this returned "" forever and the game loop hot-spun.
fn prompt<'a, T>(s: T) -> Option<String>
where
    T: Into<Cow<'a, str>>,
{
    print!("{}: \x1b[K", s.into());
    stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(input.trim().to_owned()),
    }
}
```

- [ ] Name-entry loop (lines 21-28): change the binding to

```rust
        let Some(player) = prompt(format!(
            "Enter player {} (or blank to finish)",
            player_names.len() + 1
        )) else {
            return;
        };
```

  (EOF during name entry = quit the repl entirely; a blank line still `break`s to start the game, unchanged.)
- [ ] Game-loop prompt (line 98): change to

```rust
                let Some(input) = prompt(ansi(&transform(&[Node::Player(current_player)], &players)))
                else {
                    return;
                };
```

- [ ] Add the render-refresh helper (place it next to `output_logs`, after the `repl` fn):

```rust
/// :undo/:load replace `game` without touching the cached renders; re-request
/// them so the board and command spec shown next iteration match the state the
/// next Play will actually be built from (ls F20).
fn refresh_renders<T: Requester>(
    client: &mut T,
    game: &GameResponse,
    public_render: &mut PubRender,
    player_renders: &mut Vec<PlayerRender>,
) {
    match client
        .request(&Request::Status {
            game: game.state.clone(),
        })
        .unwrap()
    {
        Response::Status {
            public_render: new_public_render,
            player_renders: new_player_renders,
            ..
        } => {
            *public_render = new_public_render;
            *player_renders = new_player_renders;
        }
        Response::SystemError { message } => panic!("{}", message),
        r => panic!("unexpected response to status request: {:?}", r),
    }
}
```

  (The `.unwrap()` on the requester call and the panics match the repl's existing dev-toy error style — ls F23 disposition.)
- [ ] Wire it into `:load` (lines 112-115) and the success branch of `:undo` (lines 116-119):

```rust
                    ":load" => {
                        let file = File::open("game.json").expect("could not open file");
                        game = serde_json::from_reader(file).expect("could not read file JSON");
                        refresh_renders(client, &game, &mut public_render, &mut player_renders);
                    }
                    ":undo" | ":u" => {
                        if let Some(u) = undo_stack.pop() {
                            game = u;
                            refresh_renders(client, &game, &mut public_render, &mut player_renders);
                        } else {
```

  (the `else` branch printing "No undos available" is unchanged.)
- [ ] ls F30 — line 59: change `let mut undo_stack: Vec<GameResponse> = vec![game.clone()];` to `let mut undo_stack: Vec<GameResponse> = vec![];`. The pre-play state is already pushed on each successful play (line 152), so the first `:undo` now correctly hits the "No undos available" branch.
- [ ] ls F26 — line 147: change `if remaining_input.trim() != "" {` to `if !remaining_input.trim().is_empty() {`.
- [ ] ls F23 — line 55: change `_ => panic!("wrong reponse"),` to `r => panic!("unexpected response to new game request: {:?}", r),` and line 167: `_ => panic!("unexpected response"),` to `r => panic!("unexpected response to play request: {:?}", r),`.
- [ ] Run: `cargo test -p brdgme_cmd` (all existing tests pass; no new tests), then `cargo clippy -p brdgme_cmd --all-targets -- -D warnings` and `cargo fmt --all -- --check`.
- [ ] Optional manual smoke (dev machine, builds one game crate): `printf 'a\nb\n' | cargo run -p tic-tac-toe-2 --bin tic_tac_toe_2_repl` — pre-fix this spins forever printing empty-command errors after the names are consumed; post-fix it exits promptly when stdin is exhausted.
- [ ] Commit: `git add rust/lib/cmd/src/repl.rs` ; message: `fix(cmd): repl EOF exits, undo/load refresh renders, empty undo stack (ls F20 ls F22 ls F26 ls F30, WP-06)`

---

### Task 4: delete bot_cli dead code; rand_bot main.rs/comment nits (ls F21 + ls F44 + ls F45)

**Problem (restated):** `bot_cli::cli` (bot_cli.rs:22-44, four runtime `.unwrap()`s) and `pub type Response` (line 20) have zero callers workspace-wide — the sole consumer of the module is `rand_bot`, which uses only the `Request` struct (rand_bot lib.rs:107). Riders in `rand_bot`: `extern crate brdgme_rand_bot;` (main.rs:1) is meaningless under edition 2024 (ls F44), and the comment at lib.rs:98-101 is line-wrap-mangled (`// / Most bots...`) and claims bots "just want to use `brdgme_cmd::bot_cli`" — which no bot does (ls F45).

**Disposition of the recommendation's two options (adjusted):** keep `Request` in `brdgme_cmd::bot_cli` and delete only `cli`/`Response`, rather than moving `Request` into `rand_bot`. Why: `Request` is the bot wire contract as seen from the platform side — `brdgme_cmd` is where the other wire types live (`api.rs`), and moving it into one bot implementation would invert the dependency direction for any future bot. It also keeps this task out of `rand_bot/src/lib.rs`'s logic, which WP-07 is actively editing (conflict avoidance); only the comment block changes there.

**Files:**
- Modify: `rust/lib/cmd/src/bot_cli.rs`, `rust/lib/rand_bot/src/main.rs`, `rust/lib/rand_bot/src/lib.rs` (comment only)

**Steps:**

- [ ] Replace the entire contents of `rust/lib/cmd/src/bot_cli.rs` with:

```rust
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;

/// Wire request for bot command generation. Consumed by `rand_bot`; the
/// generic CLI runner that used to live here was dead code (WP-06 ls F21).
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub player: usize,
    pub player_state: String,
    pub players: Vec<String>,
    pub command_spec: CommandSpec,
    pub game_id: Option<String>,
}
```

  (This drops `cli`, `Response`, and the now-unused imports `std::fmt::Debug`, `std::io::{Read, Write}`, `DeserializeOwned`, `Gamer`, `Botter`. The struct definition is byte-identical, so the JSON wire shape is untouched.)
- [ ] `rust/lib/rand_bot/src/main.rs`: delete line 1 (`extern crate brdgme_rand_bot;`). The `brdgme_rand_bot::cli(...)` path call on line 6 resolves without it on edition 2024.
- [ ] `rust/lib/rand_bot/src/lib.rs`: replace the mangled comment block (lines 98-101) with a doc comment on `cli` that no longer references the deleted API:

```rust
/// Reads a `bot_cli::Request` from `input` and writes generated commands to
/// `output`. Only `command_spec` and `players` are used - RandBot doesn't need
/// game state, so it works with arbitrary games.
pub fn cli<I, O>(input: I, output: &mut O)
```

  Do NOT touch anything else in this file (the unwraps at lib.rs:50/84/107 and the join at lib.rs:93 are WP-07's ls F43/F41).
- [ ] Run: `cargo test -p brdgme_cmd` and `cargo test -p brdgme_rand_bot` — pass (no behavior change; rand_bot has no tests, its build is the check). Then `cargo check -p tools-fuzz` if that package name exists — determine it via `grep -m1 '^name' tools/fuzz/Cargo.toml` — fuzz depends on `brdgme_rand_bot` and must still build.
- [ ] `cargo clippy -p brdgme_cmd --all-targets -- -D warnings`, `cargo clippy -p brdgme_rand_bot --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/cmd/src/bot_cli.rs rust/lib/rand_bot/src/main.rs rust/lib/rand_bot/src/lib.rs` ; message: `refactor(cmd): delete dead bot_cli runner, rand_bot extern/comment nits (ls F21 ls F44 ls F45, WP-06)`

---

### Task 5: local requester child exit status; redundant serde default (ls F29 + ls F27)

**Problems (restated):**

- ls F29 — `LocalRequester::request` (local.rs:35-41) never checks the child's `ExitStatus`; a crashed child (empty stdout) surfaces as `RequestError::Parse` ("EOF while parsing a value") instead of naming the real failure.
- ls F27 — `#[serde(default)]` on `seed: Option<u64>` (api.rs:14) is redundant; serde already defaults missing `Option` fields to `None` for both the derive and all container shapes used here.

**Fix (re-derived, confirms both recommendations):** new `RequestError::ChildExit` variant checked after `wait_with_output` (stderr passthrough stays first, so the child's own diagnostics still print before the error); delete the attribute. A lock-in test pins the `seed`-absent deserialization so the attribute removal is provably a no-op.

**Files:**
- Modify: `rust/lib/cmd/src/requester/local.rs`, `rust/lib/cmd/src/requester/error.rs`, `rust/lib/cmd/src/api.rs`
- Tests: inline in `local.rs` and in api.rs's existing `mod tests`

**Steps:**

- [ ] Write the failing test. Append to `rust/lib/cmd/src/requester/local.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failing_child_reports_exit_status_not_json_error() {
        // ls F29: a child that dies without output must be reported as an
        // exit-status failure, not a JSON parse error on empty stdout.
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join("brdgme_cmd_local_requester_fail.sh");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            // Consume stdin first so the request write never races the exit
            // (a closed pipe would surface as an IO error instead).
            f.write_all(b"#!/bin/sh\ncat >/dev/null\nexit 3\n").unwrap();
        }
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();

        let mut requester = LocalRequester::new(path.as_os_str());
        let err = requester.request(&Request::PlayerCounts).unwrap_err();
        let _ = std::fs::remove_file(&path);
        match err {
            RequestError::ChildExit { status } => assert_eq!(Some(3), status.code()),
            e => panic!("expected ChildExit, got {:?}", e),
        }
    }
}
```

- [ ] Run: `cargo test -p brdgme_cmd requester::local` — expected: compile FAILURE (`ChildExit` does not exist). Add the variant to `rust/lib/cmd/src/requester/error.rs` (inside `RequestError`, after `Stdin`):

```rust
    #[error("child process exited with {status}")]
    ChildExit { status: std::process::ExitStatus },
```

  Re-run — expected: the test now FAILS at the `match` with `expected ChildExit, got Parse { .. }` (pre-fix behavior: empty stdout parsed as JSON).
- [ ] Implement in `rust/lib/cmd/src/requester/local.rs`: after the stderr passthrough (lines 37-39) and before the final parse (line 41), insert:

```rust
        if !output.status.success() {
            return Err(RequestError::ChildExit {
                status: output.status,
            });
        }
```

- [ ] Run: `cargo test -p brdgme_cmd requester::local` — PASSES.
- [ ] ls F27 lock-in test — add to the existing `mod tests` at the bottom of `rust/lib/cmd/src/api.rs`:

```rust
    #[test]
    fn new_request_without_seed_deserializes_to_none() {
        // Pins that removing #[serde(default)] from `seed` changes nothing:
        // serde defaults a missing Option field to None regardless (ls F27).
        match serde_json::from_str::<Request>(r#"{"New":{"players":2}}"#).unwrap() {
            Request::New { players, seed } => {
                assert_eq!(2, players);
                assert_eq!(None, seed);
            }
            r => panic!("expected New, got {:?}", r),
        }
    }
```

  Run it (`cargo test -p brdgme_cmd new_request_without_seed`) — PASSES before the change. Then delete line 14 of `rust/lib/cmd/src/api.rs` (`#[serde(default)]`, leaving `seed: Option<u64>,`). Run again — still PASSES.
- [ ] Run the full crate: `cargo test -p brdgme_cmd`; `cargo clippy -p brdgme_cmd --all-targets -- -D warnings`; `cargo fmt --all -- --check`.
- [ ] Final package gate: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass (this also runs the workspace-minus-web clippy/test that covers every game bin against the modified crate).
- [ ] Commit: `git add rust/lib/cmd/src/requester/local.rs rust/lib/cmd/src/requester/error.rs rust/lib/cmd/src/api.rs` ; message: `fix(cmd): report child exit status in local requester, drop redundant serde default (ls F27 ls F29, WP-06)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| ls F19 warp handler unwrap | major | `unwrap_or_else` → `Response::SystemError`; delete unused `impl Reject` (or use it) | **Confirmed** (Task 1) | Re-traced: `Err` reachable via any malformed `game` string (gamer.rs:28,37,41,45). SystemError is the protocol's existing in-band error channel (used by gamer.rs and parsed by web). Rejection wiring deliberately deleted, not completed — rejections bypass the JSON contract. Refinement: `RequestError::Parse` Display gains `{source}` so the message is actionable; route extracted for testability. |
| ls F20 repl undo/load stale renders | minor | Re-request `Status` and adopt renders, or store renders on the undo stack | **Adjusted** (Task 3) | First option taken; the stack-storage alternative rejected because it cannot fix `:load` (no stack entry exists for a loaded file) and duplicates render state per entry. |
| ls F21 bot_cli::cli/Response dead | minor | Delete `cli`/`Response` (keep `Request`), or move `Request` into rand_bot | **Adjusted** (Task 4) | First option taken: `Request` is platform-side wire contract, belongs with `api.rs`'s types, and keeping it avoids logic churn in rand_bot lib.rs which WP-07 is editing concurrently. |
| ls F22 repl EOF hot spin | minor | `prompt` returns `Option`/`Result`, EOF = quit | **Confirmed** (Task 3) | `Option<String>`, `Ok(0) | Err(_) → None`, both call sites quit; also discharges the `read_line` unwrap at repl.rs:233. |
| ls F23 panic-heavy runtime paths | minor | gamer.rs render handlers reuse `GameResponseError`; cli.rs mirrors SystemError; leave repl panics but fix the message | **Confirmed** (Tasks 2, 3) | All sites verified live. `renders` → `Result` (no external callers). cli.rs output-write panics stay as `expect` with messages — process-boundary exception per docs/CODING.md; no error channel remains once stdout is gone. Repl panic messages given content, panics retained per the finding's own carve-out. |
| ls F26 comparison_to_empty | nit | `is_empty()` | **Confirmed** (Task 3) | repl.rs:147, mechanical. |
| ls F27 redundant `#[serde(default)]` | nit | Remove the attribute | **Confirmed** (Task 5) | No-op for serialize and deserialize; pinned by a lock-in test. |
| ls F28 no content-length limit | nit | `warp::body::content_length_limit(...)` before `body::json()` | **Confirmed** (Task 1) | 16 MiB cap. Side effect accepted and documented: requests lacking `Content-Length` now get 411 (both real clients always send it). |
| ls F29 child exit status unchecked | nit | Check `ExitStatus`, include in error | **Confirmed** (Task 5) | New `RequestError::ChildExit` after stderr passthrough; deterministic test via a stdin-draining `/bin/sh` script. |
| ls F30 first :undo silent reset | nit | Start stack empty; push pre-play state on each play | **Adjusted** (Task 3) | Half the recommendation was already implemented — the push-on-play exists at repl.rs:152; only the seeding (line 59) changes. |
| ls F44 rand_bot `extern crate` | nit | Delete the line | **Confirmed** (Task 4) | Edition 2024; path call resolves without it. |
| ls F45 rand_bot mangled comment | nit | Reword or delete alongside the bot_cli cleanup | **Confirmed** (Task 4) | Rewritten as a doc comment with no reference to the deleted runner. |

## Cross-package coordination

- **WP-71 (warp → axum, D-22):** if/when it lands, it must carry forward Task 1's semantics (SystemError mapping, body cap, the `route`-level tests port to axum handlers). Sequencing either way is safe; F19 must not wait on D-22.
- **WP-07 (rand_bot lib.rs):** Task 4 touches only the lib.rs:98-101 comment; land whichever package first and rebase the other — the hunks are disjoint from WP-07's (lib.rs:50/84/93/107).
- **WP-09 (D-36):** if the requester-boundary validation option is chosen, its bounds check lands in `handle_player_render` — after this package that function is the Task 2 version above.
- **WP-68 (term_size):** repl.rs:186 untouched here; no conflict (Task 3 edits lines 11, 21-28, 55, 59, 98, 112-128, 147, 167, 226-235 and adds a helper).
