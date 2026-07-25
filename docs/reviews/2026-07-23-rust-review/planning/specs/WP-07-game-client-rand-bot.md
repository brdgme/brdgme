# WP-07: game_client and rand_bot

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Make `brdgme_game_client`'s reliability guarantees actually hold and give it typed errors, then clean up `brdgme_rand_bot`'s panics and dependency hygiene. The major is ls F31: the crate documents timeout-retry but never enforces a timeout, so the operator (which uses a bare `reqwest::Client::new()`) can hang a reconcile worker forever on a game pod that accepts and stalls. Riders: `anyhow`-in-a-library replaced by a `thiserror` enum (ls F32), a too-narrow retry predicate (ls F33), unvalidated `version_name` in the Host header (ls F35), `fetch_game_data`'s 5 sequential round trips (ls F36), a documented-flaky retry test (ls F37), rand_bot's unused `chrono` dependency (ls F40 = dp F10, one finding), the token-join double-space inconsistency (ls F41), the unneeded `http-server` feature pull (ls F42), and degenerate-spec panics (ls F43).

**Architecture — how these crates work (read this before editing):**

- Crate `rust/lib/game_client` (package `brdgme_game_client`, single file `src/lib.rs`, 738 lines incl. tests): the one HTTP client every in-cluster caller (web, bot, operator) MUST use to reach game services, because the KEDA HTTP interceptor routes purely on the `Host: {version_name}.games.internal` header which this crate sets (lib.rs:1-5, 54, 60). Public surface: `request()` (lib.rs:113), `render()`/`pub_render()`/`player_render()` (lib.rs:149/162/174), `fetch_game_data()` (lib.rs:212), and the data structs `RenderResponse`/`GameData`. Internals: `RetryConfig` (private, lib.rs:17-33), `backoff_delay` (lib.rs:39), `send_with_retry` (lib.rs:47-89, retry predicate at :80), `request_with_config` (lib.rs:91-110). The retry policy retries only transport failures, never any received HTTP response (4xx/5xx/SystemError are game-logic errors) — that invariant is test-locked (lib.rs:384-427) and MUST be preserved.
- **The crate takes the caller's `reqwest::Client` and sets no timeout of its own.** Caller configs, verified live: `web` builds one client, connect 5s / total 10s (web/src/main.rs:32-36); `bot` builds `game_http`, connect 5s / total 60s (bot/src/main.rs:786-790 — 60s is deliberate: the KEDA interceptor holds the request open while scaling a game pod from zero, so request time includes cold start); **operator uses `reqwest::Client::new()` with no timeout at all** (operator/src/controller.rs:230). Any fix that *replaces* the client-level timeout (e.g. `RequestBuilder::timeout`, which reqwest documents as overriding the client timeout for that request) would silently change web's 10s and bot's 60s — the fix must be a *ceiling* (min-semantics), not a replacement. See Task 2.
- **Callers of `brdgme_game_client` (complete workspace audit, 2026-07-25):**
  - Cargo dependents: `web` (optional, `ssr` feature, `sentry` feature on — web/Cargo.toml:69,124), `bot` (bot/Cargo.toml:26), `operator` (operator/Cargo.toml:19). No game crate or tool depends on it.
  - Call sites and how each consumes the error (relevant to the ls F32 return-type change):
    - `operator/src/controller.rs:53` — `.map_err(|e| Error::GameService(format!("{e:#}")))`. Display-based; compiles unchanged with a `thiserror` error.
    - `bot/src/main.rs:379` — result is `match`ed; `Err(e) => e.to_string()` (main.rs:418). Display-based; unchanged.
    - `bot/src/main.rs:499` — `fetch_game_data(...).await.context("Failed to fetch game data")?` into an `anyhow::Result`. `anyhow::Context` works on any `Result<T, E: StdError + Send + Sync + 'static>`; unchanged. `bot/src/main.rs:483` uses the `GameData` struct (type unchanged).
    - `web/src/rules.rs:261,279` — `.await?` inside fns returning `anyhow::Result`; `anyhow::Error: From<E: StdError + Send + Sync + 'static>`; unchanged.
    - `web/src/game/server_fns.rs:263,420,621,804` — `.map_err(internal("..."))`; `internal<E: Display>` (web/src/error.rs:7). Unchanged.
    - `web/src/email/commands.rs:1014` — `.map_err(|e| CommandError::Internal(anyhow::anyhow!("undo: fetch status: {e}")))`. Display-based; unchanged.
    - `web/src/email/notify.rs:83` — result `match`ed Ok/Err, best-effort. Unchanged.
    - `web/src/game/mod.rs:114-124` — **the one breaking site**: `client::request(...).await?` inside `execute_command`, which returns `Result<(), ExecuteCommandError>` where `ExecuteCommandError` (mod.rs:67-76) only has `Other(#[from] anyhow::Error)`. `?` does not chain two `From` hops (`GameClientError → anyhow::Error → ExecuteCommandError`), so this becomes a compile error — fixed with a one-line `.map_err(anyhow::Error::from)` in Task 1. It is a compile error, not a silent break; no web test asserts on the exact error string of a game-client failure (the `system_error_propagated_and_no_db_write` test at mod.rs:830 asserts only `result.is_err()`).
- Crate `rust/lib/rand_bot` (package `brdgme_rand_bot`, `src/lib.rs` 136 lines + 6-line `src/main.rs`): a stdin/stdout random bot. `spec_to_command` (lib.rs:24-87) walks a `command::Spec` producing tokens; `commands` (lib.rs:89-96, private) joins them into one `BotCommand`; `cli` (lib.rs:102-114) reads a `brdgme_cmd::bot_cli::Request` and writes `Vec<BotCommand>` JSON; `Botter` impl + `fuzz` helper (lib.rs:116-135). External consumers: `tools/fuzz` calls `brdgme_rand_bot::spec_to_command(...).join("")` directly (tools/fuzz/src/lib.rs:349) — the ONLY external caller of any rand_bot fn besides `main.rs`'s `cli`. Game `*_fuzz` bins go through `brdgme_fuzz`, not rand_bot. `spec_to_command`'s signature must not change; its behavior on degenerate specs (currently panics) getting more lenient is strictly an improvement for the fuzz driver.
- `brdgme_cmd`'s `default` feature = `http-server` = warp + tokio + sentry (lib/cmd/Cargo.toml:24-26). Its `api`/`bot_cli`/`cli`/`repl`/`requester` modules are NOT feature-gated (lib/cmd/src/lib.rs:7-14); only `http` is. `game_client` already depends on it with `default-features = false` and builds — proof rand_bot can too (ls F42).

**Tech Stack:** Rust 1.97.0 (edition 2024), workspace at `/home/beefsack/Development/brdgme/rust`. Crates touched: `brdgme_game_client`, `brdgme_rand_bot`, plus one-line caller fix in `web`. No new dev-dependencies: `game_client` already dev-deps `axum` 0.8.9 + `tokio` (macros, rt-multi-thread, net) and its tests already use the accept-and-never-respond TcpListener pattern (lib.rs:429-476) — the new timeout test reuses it, no HTTP-stub crate needed. New runtime deps for `game_client`: `thiserror = "2.0.18"` (version already in Cargo.lock via `brdgme_cmd`) replacing `anyhow`; tokio gains the `macros` feature (for `tokio::try_join!`, already in the lock).

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p brdgme_game_client`, `cargo test -p brdgme_rand_bot`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints). The one web check is `cargo clippy -p web --all-targets --features ssr -- -D warnings` (matches CI; needed once for the Task 1 caller fix).
- Each task ends with `cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- docs/CODING.md "no panicking code in runtime paths" applies to `game_client` fully (it already complies) and to rand_bot's generation paths; rand_bot's `cli` stdin/stdout boundary follows the WP-06 precedent (`expect` with a real message at a process boundary that has no in-band error channel).
- **Wire compatibility:** nothing here may change how any `brdgme_cmd::api::Request`/`Response` serializes; every change is client-side control flow, error typing, or Cargo metadata.
- **Retry-policy invariant:** received HTTP responses (any status) are never retried. The Task 2 predicate widening applies only to transport errors from `send()`.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests need the containers the script provides; failures without them are pre-existing, backlog #40).

**Non-Goals (owned elsewhere — do NOT touch):**

- ls F34 / dp F14 / bo F17 (`serde_yaml` 0.9 deprecated) — WP-70, blocked on D-21. `serde_yaml = "0.9"` stays in game_client's Cargo.toml and `json_to_yaml` keeps calling it; only its *error type* is wrapped by the new enum here.
- ls F44 (rand_bot `main.rs` `extern crate` line) and ls F45 (the mangled comment at rand_bot lib.rs:98-101) — WP-06 Task 4. Do not edit `rand_bot/src/main.rs` at all, and do not touch lines 98-101; this package's lib.rs edits (lines 49-51, 84, 93, 107-113, new test module) are disjoint from WP-06's.
- `bot_cli::cli`/`Response` dead code in `brdgme_cmd` (ls F21) — WP-06. rand_bot keeps consuming `bot_cli::Request` from `brdgme_cmd`.
- tools/fuzz (`spec_to_command` call, its own join, num_cpus, hang bug) — WP-63. The ls F41 recommendation's "have tools/fuzz call RandBot's commands" half is deferred to WP-63 (see Cross-package coordination); this package only makes rand_bot's join match fuzz's.
- Workspace-deps restructuring (dp F1-F3) — WP-64. The Cargo edits here are exactly the ones the findings name (delete `chrono`, `default-features = false`, swap `anyhow`→`thiserror`, add tokio `macros`), in the per-crate manifests as they exist today.
- Operator/bot/web client *construction* (timeouts, connect timeouts) — the operator's bare `Client::new()` is bounded by this package's crate-level ceiling; giving the operator its own tighter client config is WP-62's discretion (its findings don't require it).
- warp→axum (WP-71), sentry feature trim (dp F12, WP-67): the `sentry` optional dep and `#[cfg(feature = "sentry")]` block in `send_with_retry` (lib.rs:63-74) are untouched.

**Snapshot drift:** None. `diff` of every touched file (`lib/game_client/src/lib.rs`, `lib/game_client/Cargo.toml`, `lib/rand_bot/src/lib.rs`, `lib/rand_bot/src/main.rs`, `lib/rand_bot/Cargo.toml`) against `/home/beefsack/Development/brdgme-review-snapshot/rust` (commit f8763a5) is empty — verified 2026-07-25. All line numbers in this spec are live-file line numbers and match the findings' citations. (WP-06, if it lands first, edits `rand_bot/src/lib.rs:98-102` and `rand_bot/src/main.rs` — re-check drift on those two files at implementation time; the hunks are disjoint.)

**Re-derivation of ls F31 (the major) — verified against live source:** `send_with_retry` keys retry on `e.is_timeout()` (lib.rs:80) and the `RetryConfig` doc says timeouts are among the "transient transport failures" retried (lib.rs:12-15), but nothing in the crate ever *sets* a timeout — `client.post(uri)` (lib.rs:58-61) inherits whatever the caller's `Client` has. For web (10s) and bot (60s) the documented behavior holds by luck of caller config; for the operator (`Client::new()`, no timeout — controller.rs:230) a game pod that accepts the connection and never responds blocks that reconcile call **forever** (kube-rs runs reconciles concurrently, but the stuck GameVersion is never reconciled again and the task leaks). The finding's recommendation ("set a default per-request timeout inside `send_with_retry`, e.g. `request_builder.timeout(...)`, possibly on `RetryConfig`") is directionally right but the named mechanism is **wrong**: `reqwest::RequestBuilder::timeout` *replaces* the client-level timeout for that request, so any crate default would silently override web's 10s and bot's 60s — exactly the silent caller breakage this package must avoid. The correct mechanism is `tokio::time::timeout(config.request_timeout, ...)` around the send and the body read: min-semantics (the tighter of caller timeout and crate ceiling wins), so web still fails at 10s, bot at 60s, and the operator is newly bounded at the crate ceiling. Ceiling chosen: **90s** — above bot's deliberate 60s cold-start allowance so the ceiling never preempts any configured caller, low enough that a wedged operator reconcile recovers within ~5 minutes worst-case (3 attempts x 90s + backoffs). Elapsed ceilings are classified as retryable timeouts, making the documented timeout-retry path real for the operator.

---

### Task 1: typed error enum replacing anyhow (ls F32, minor) + one-line caller fix

**Problem (restated):** every other `lib/` crate uses `thiserror`; `game_client` is the sole `anyhow` library. Transport failures, non-2xx statuses (lib.rs:102), `Response::SystemError` (lib.rs:107), response-variant mismatches (lib.rs:170,190,234,254,268,279,289), state-JSON/YAML failures (lib.rs:206-210), and the missing-player-render case (lib.rs:238) all flatten into indistinguishable strings — callers cannot branch on kind without string matching.

**Fix (re-derived, confirms the finding; variant list extended):** a public `GameClientError` enum. The finding's suggested variants (Transport, HttpStatus, SystemError, UnexpectedResponse, Parse) are kept and extended with the variants the actual code paths need: `Timeout` and `InvalidVersionName` (created by Task 2), `NoPlayerRender`, `StateJson`, `StateYaml`. Display messages embed enough detail that the operator's `format!("{e:#}")` logging (which loses anyhow's chain rendering) stays actionable. Every public fn returns `Result<T, GameClientError>`. Exactly one caller needs an edit (see Architecture audit): `web/src/game/mod.rs` gets `.map_err(anyhow::Error::from)`.

**Files:**
- Modify: `rust/lib/game_client/src/lib.rs`, `rust/lib/game_client/Cargo.toml`, `rust/web/src/game/mod.rs` (one line)

**Interfaces:** public fn signatures change from `anyhow::Result<T>` to `Result<T, GameClientError>`; `GameClientError` is a new public type. `RenderResponse`, `GameData`, and all parameter lists are unchanged.

**Steps:**

- [ ] `rust/lib/game_client/Cargo.toml`: delete line 9 (`anyhow = "1.0.103"`) and add (keeping the list alphabetical):

```toml
thiserror = "2.0.18"
```

- [ ] In `rust/lib/game_client/src/lib.rs`, replace the import at line 7 (`use anyhow::{Context, Result, anyhow};`) with nothing (the other imports stay), and add the error enum directly below the crate doc comment (after line 10):

```rust
/// Typed errors for game-service calls, replacing the crate's previous
/// anyhow strings so callers can branch on kind (ls F32, WP-07). Display
/// messages are self-contained (they embed the underlying cause) because
/// two callers log via `Display` only.
#[derive(Debug, thiserror::Error)]
pub enum GameClientError {
    /// `version_name` is interpolated into the Host header the KEDA
    /// interceptor routes on; reject non-DNS-label names up front (ls F35).
    #[error("invalid game version name {name:?}: must be a DNS label (ASCII alphanumeric and '-', 1-63 chars, no leading/trailing '-')")]
    InvalidVersionName { name: String },
    #[error("transport error calling game service: {0}")]
    Transport(#[from] reqwest::Error),
    /// The crate-level per-attempt ceiling fired (ls F31). Callers with a
    /// tighter `reqwest::Client` timeout see `Transport` instead.
    #[error("game service request timed out after {after:?}")]
    Timeout { after: std::time::Duration },
    #[error("game service returned {status}: {body}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("error parsing game service response: {source}; body: {body}")]
    ParseResponse {
        body: String,
        #[source]
        source: serde_json::Error,
    },
    /// The service reported an in-band `Response::SystemError`.
    #[error("game service system error: {message}")]
    SystemError { message: String },
    #[error("unexpected response to {request} request")]
    UnexpectedResponse { request: &'static str },
    #[error("no player render for position {player}")]
    NoPlayerRender { player: usize },
    #[error("invalid JSON in game state: {0}")]
    StateJson(#[source] serde_json::Error),
    #[error("failed to serialize state as YAML: {0}")]
    StateYaml(#[from] serde_yaml::Error),
}
```

  (Only `Transport` and `StateYaml` get `#[from]` — `ParseResponse` and `StateJson` both wrap `serde_json::Error` and must be constructed explicitly.)
- [ ] Re-type the request pipeline. `send_with_retry` (lib.rs:47-89): change the return type to `Result<reqwest::Response, GameClientError>` and the terminal error returns to `Err(e.into())` (body otherwise unchanged in this task; Task 2 rewrites the match). `request_with_config` (lib.rs:91-110) becomes:

```rust
async fn request_with_config(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
    config: &RetryConfig,
) -> Result<Response, GameClientError> {
    let res = send_with_retry(client, uri, version_name, request, config).await?;
    let status = res.status();
    let body = res.text().await.map_err(GameClientError::Transport)?;
    if !status.is_success() {
        return Err(GameClientError::HttpStatus { status, body });
    }
    let resp: Response = serde_json::from_str(&body)
        .map_err(|source| GameClientError::ParseResponse { body, source })?;
    match resp {
        Response::SystemError { message } => Err(GameClientError::SystemError { message }),
        other => Ok(other),
    }
}
```

- [ ] Re-type the public fns — mechanical, same shapes:
  - `request` (lib.rs:113): `-> Result<Response, GameClientError>`.
  - `render`/`pub_render`/`player_render` (lib.rs:149/162/174): `-> Result<RenderResponse, GameClientError>`; the two `Err(anyhow!("invalid response type"))` arms (lib.rs:170, 190) become `Err(GameClientError::UnexpectedResponse { request: "PubRender" })` and `{ request: "PlayerRender" }`.
  - `json_to_yaml` (lib.rs:206-210): `-> Result<String, GameClientError>`, body:

```rust
fn json_to_yaml(json: &str) -> Result<String, GameClientError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(GameClientError::StateJson)?;
    serde_yaml::to_string(&value).map_err(GameClientError::from)
}
```

  - `fetch_game_data` (lib.rs:212): `-> Result<GameData, GameClientError>`; its five `anyhow!` sites become `UnexpectedResponse { request: "Status" | "DataDocs" | "BasicStrategy" | "AdvancedStrategy" | "Rules" }` (lib.rs:234, 254, 268, 279, 289) and `NoPlayerRender { player }` for the `.ok_or_else` at lib.rs:238 (use `.ok_or(GameClientError::NoPlayerRender { player })`).
- [ ] Update the one existing test that asserts on anyhow rendering — `test_no_retry_on_http_error_response`, lib.rs:417-421: replace

```rust
        let err = format!("{:#}", resp.unwrap_err());
        assert!(
            err.contains("500"),
            "error must include the HTTP status, got: {err}"
        );
```

  with

```rust
        let err = resp.unwrap_err();
        assert!(
            matches!(err, GameClientError::HttpStatus { status, .. } if status.as_u16() == 500),
            "expected HttpStatus 500, got: {err}"
        );
```

- [ ] Add variant-mapping tests (append inside the existing `mod tests`; the axum mock helpers at lib.rs:641-699 are already there):

```rust
    #[tokio::test]
    async fn test_system_error_maps_to_typed_variant() {
        // ls F32: in-band SystemError must surface as a matchable kind, not
        // a bare string.
        let app = Router::new().route(
            "/",
            post(|Json(_): Json<Request>| async move {
                Json(Response::SystemError {
                    message: "state exploded".to_string(),
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = pub_render(&client, &uri, "test-game-1", "g".to_string())
            .await
            .unwrap_err();
        match err {
            GameClientError::SystemError { message } => assert_eq!("state exploded", message),
            e => panic!("expected SystemError, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_wrong_variant_maps_to_unexpected_response() {
        let app = Router::new().route(
            "/",
            post(|Json(_): Json<Request>| async move {
                Json(Response::Rules {
                    rules: "not a render".to_string(),
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = pub_render(&client, &uri, "test-game-1", "g".to_string())
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                GameClientError::UnexpectedResponse {
                    request: "PubRender"
                }
            ),
            "got {:?}",
            err
        );
    }
```

- [ ] Fix the one breaking caller. `rust/web/src/game/mod.rs`, the `client::request(...)` call in `execute_command` (lines 114-124): change the terminal `.await?;` to

```rust
    .await
    .map_err(anyhow::Error::from)?;
```

  Why: `ExecuteCommandError` (mod.rs:67-76) converts only from `anyhow::Error`, and `?` will not chain `GameClientError → anyhow::Error → ExecuteCommandError`. Routing through `Other(anyhow::Error)` preserves today's semantics exactly (a game-service failure is an internal error, not a `UserError`/`Conflict`). All other callers compile unchanged (audit in Architecture).
- [ ] Run: `cargo test -p brdgme_game_client` — all pre-existing tests plus the two new ones PASS. Then `cargo clippy -p brdgme_game_client --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
- [ ] Caller compile checks: `cargo check -p operator`, `cargo check -p brdgme-bot` (confirm the bot package name via `grep -m1 '^name' bot/Cargo.toml` and substitute), `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- [ ] Commit: `git add rust/lib/game_client/src/lib.rs rust/lib/game_client/Cargo.toml rust/web/src/game/mod.rs` ; message: `refactor(game_client): thiserror GameClientError replaces anyhow (ls F32, WP-07)`

---

### Task 2: crate-enforced request timeout ceiling, wider retry predicate, version_name validation (ls F31 MAJOR + ls F33 + ls F35)

**Fix (re-derived — see the F31 re-derivation block above for why NOT `RequestBuilder::timeout`):**

1. `RetryConfig` gains `request_timeout: Duration` (default 90s). Each send attempt and the body read are wrapped in `tokio::time::timeout` — a *ceiling*: callers' tighter client timeouts still fire first as `reqwest` timeout errors (retryable, unchanged), and only the operator's unbounded client newly hits the ceiling (`GameClientError::Timeout`, retryable).
2. ls F33: the retry predicate `e.is_connect() || e.is_timeout()` misses a connection accepted then reset mid-request (pod killed between accept and response — plausible exactly during KEDA scale-down). Add `e.is_request()`. Safe because: retries are bounded (`max_attempts` 3); game-service requests are stateless request/response (the bot's own comment at bot/src/main.rs:375-377: "`Play` is stateless (returns the new state but doesn't persist)"), so re-sending is harmless; and non-transport failures (4xx/5xx, malformed body) never reach this predicate — they arrive as `Ok(res)` and are returned immediately. Builder errors (`is_builder()`) remain non-retryable.
3. ls F35: validate `version_name` as a DNS label at the top of `request_with_config` (covers every public entry point), returning `InvalidVersionName` instead of an opaque reqwest builder error from deep inside header construction.

Behavior before → after:

- Operator calling a hung game pod: hangs forever → fails with `Timeout` after 3 attempts x 90s (+ ≤ ~4.5s backoff), reconcile requeues via its normal `error_policy`.
- web/bot calling a hung pod: unchanged (their 10s/60s client timeouts still fire first, still retried as timeouts).
- Connection reset mid-request during pod churn: previously failed immediately → now retried up to `max_attempts`.
- `version_name` with a `.`, `_`, space, or empty: previously an opaque reqwest error (or a mis-routed Host) → now a clear `InvalidVersionName` before any connection. All live names are k8s object names / `game_versions.name` values (DNS labels like `acquire-1`); no legitimate name is rejected.

**Files:**
- Modify: `rust/lib/game_client/src/lib.rs` only

**Steps:**

- [ ] Extend `RetryConfig` (lib.rs:17-33) — add the field and default, and update the struct doc comment (lib.rs:12-15) to state the crate-enforced ceiling:

```rust
/// Bounded retry policy for transient transport failures (connect-refused,
/// timeouts, connections reset mid-request) talking to the game service.
/// Does not retry on any received HTTP response, including non-2xx status -
/// those are game-logic errors, not transport failures.
///
/// `request_timeout` is a crate-enforced per-attempt ceiling applied with
/// `tokio::time::timeout`, NOT `reqwest`'s per-request timeout: reqwest's
/// would *replace* the caller's client-level timeout (web 10s, bot 60s),
/// whereas a ceiling composes - the tighter of the two always wins. It
/// exists so the guarantee holds even for callers that configure no client
/// timeout at all (the operator, ls F31). It must stay above 60s: the KEDA
/// interceptor holds requests open while a game pod cold-starts, and bot
/// deliberately allows 60s for that.
#[derive(Debug, Clone)]
struct RetryConfig {
    base_delay: Duration,
    multiplier: f64,
    cap: Duration,
    max_attempts: u32,
    request_timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_millis(300),
            multiplier: 2.0,
            cap: Duration::from_secs(3),
            max_attempts: 3,
            request_timeout: Duration::from_secs(90),
        }
    }
}
```

- [ ] Rewrite the send/match tail of `send_with_retry` (the loop body from lib.rs:76-87; the builder construction and sentry block above it are untouched):

```rust
        match tokio::time::timeout(config.request_timeout, request_builder.send()).await {
            Ok(Ok(res)) => return Ok(res),
            Ok(Err(e)) => {
                // Transport-level failures only ever arrive here (a received
                // HTTP response is Ok regardless of status). Retry the
                // transient kinds: connect-refused (KEDA scale-from-zero),
                // timeouts, and request-phase failures such as a connection
                // reset between accept and response (pod churn, ls F33).
                // Requests are stateless server-side, so re-sending is safe.
                let retryable = e.is_connect() || e.is_timeout() || e.is_request();
                attempt += 1;
                if !retryable || attempt >= config.max_attempts {
                    return Err(e.into());
                }
            }
            // Crate-level ceiling fired (ls F31): the caller's Client has no
            // (or a looser) timeout. Retryable, like any other timeout.
            Err(_elapsed) => {
                attempt += 1;
                if attempt >= config.max_attempts {
                    return Err(GameClientError::Timeout {
                        after: config.request_timeout,
                    });
                }
            }
        }
        let delay = backoff_delay(attempt - 1, config);
        tokio::time::sleep(delay).await;
```

  (The `let delay` / `sleep` pair moves from inside the old `Err` arm to the loop tail — both surviving arms fall through to it.)
- [ ] Bound the body read in `request_with_config` — replace the `let body = ...` line from Task 1 with:

```rust
    let body = tokio::time::timeout(config.request_timeout, res.text())
        .await
        .map_err(|_| GameClientError::Timeout {
            after: config.request_timeout,
        })?
        .map_err(GameClientError::Transport)?;
```

  Why: a server that returns headers then stalls the body would otherwise still hang an unbounded caller.
- [ ] ls F35 — add the validator (above `send_with_retry`) and call it first thing in `request_with_config`:

```rust
/// The interceptor routes on `Host: {version_name}.games.internal`; a
/// malformed name would otherwise fail deep inside reqwest's header builder
/// with an opaque error (or, with a '.', route to the wrong backend). All
/// legitimate names are k8s object names / game_versions.name values, i.e.
/// DNS labels (ls F35).
fn validate_version_name(name: &str) -> Result<(), GameClientError> {
    let ok = !name.is_empty()
        && name.len() <= 63
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(GameClientError::InvalidVersionName {
            name: name.to_string(),
        })
    }
}
```

  and in `request_with_config`, as the first line: `validate_version_name(version_name)?;`
- [ ] Update every `RetryConfig` literal in the test module to carry the new field: `tiny_config` (lib.rs:313-320) gets `request_timeout: Duration::from_secs(90),`; the literals in `test_retry_on_connect_refused_then_success` (lib.rs:355-360), `test_backoff_delay_grows_with_attempt` (lib.rs:480-485), `test_backoff_delay_respects_cap` (lib.rs:494-499), and `test_backoff_delay_jitter_varies_within_band` (lib.rs:517-522) get the same line. (90s keeps existing tests' timing driven by their client-level timeouts, e.g. the 30ms client in `test_bounded_max_attempts_on_permanent_failure` — that test still passes unchanged, which itself proves the ceiling does not preempt tighter client timeouts.)
- [ ] Add the F31 test — the operator scenario: no client timeout, hung server, tiny ceiling. Reuses the accept-and-hold listener pattern from lib.rs:429-453 verbatim (a std-TcpListener stub per the packaging guidance would duplicate what these tests already do with tokio's listener; no new dev-deps either way):

```rust
    #[tokio::test]
    async fn test_crate_timeout_bounds_client_without_timeout() {
        // ls F31: the operator uses reqwest::Client::new() with no timeout;
        // an accepted-but-hung request must still be bounded by the crate.
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        std_listener.set_nonblocking(true).unwrap();
        let addr = std_listener.local_addr().unwrap();
        let listener = TcpListener::from_std(std_listener).unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter2 = counter.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((socket, _)) = listener.accept().await {
                    counter2.fetch_add(1, Ordering::SeqCst);
                    tokio::spawn(async move {
                        let _socket = socket;
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    });
                }
            }
        });

        // Deliberately NO client-level timeout - the operator's config.
        let client = reqwest::Client::new();
        let config = RetryConfig {
            base_delay: Duration::from_millis(5),
            multiplier: 2.0,
            cap: Duration::from_millis(20),
            max_attempts: 3,
            request_timeout: Duration::from_millis(50),
        };
        let uri = format!("http://{}", addr);
        let start = std::time::Instant::now();
        let resp = request_with_config(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
            &config,
        )
        .await;
        match resp {
            Err(GameClientError::Timeout { after }) => {
                assert_eq!(Duration::from_millis(50), after)
            }
            r => panic!("expected Timeout, got {:?}", r),
        }
        assert_eq!(
            3,
            counter.load(Ordering::SeqCst),
            "ceiling timeouts must be retried up to max_attempts"
        );
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "must not hang; elapsed={:?}",
            start.elapsed()
        );
    }
```

- [ ] Add the F33 test — reset mid-request then success, raw sockets (axum cannot script "accept then drop"):

```rust
    #[tokio::test]
    async fn test_retry_on_connection_reset_mid_request() {
        // ls F33: a connection accepted then closed before any response (pod
        // killed during scale-down) must be retried like other transient
        // transport failures.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            // First connection: accept and drop immediately (reset/EOF before
            // any response bytes - surfaces as a request error, not connect
            // or timeout).
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
            // Second connection: minimal canned HTTP response.
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut seen: Vec<u8> = vec![];
            let mut buf = [0u8; 8192];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                seen.extend_from_slice(&buf[..n]);
                if seen.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = serde_json::to_string(&Response::PubRender {
                render: PubRender {
                    pub_state: "pub".to_string(),
                    render: "render".to_string(),
                },
            })
            .unwrap();
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        });

        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let resp = request_with_config(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
            &tiny_config(),
        )
        .await;
        assert!(
            matches!(resp, Ok(Response::PubRender { .. })),
            "mid-request reset must be retried to success, got {:?}",
            resp
        );
    }
```

- [ ] Add the F35 test (no server needed — validation fails before any I/O):

```rust
    #[tokio::test]
    async fn test_invalid_version_name_rejected_before_send() {
        let client = reqwest::Client::new();
        for bad in ["", "has.dot", "under_score", "-leading", "trailing-", "has space"] {
            let err = request(
                &client,
                "http://127.0.0.1:1",
                bad,
                &Request::PlayerCounts,
            )
            .await
            .unwrap_err();
            assert!(
                matches!(err, GameClientError::InvalidVersionName { .. }),
                "{bad:?} should be rejected, got {:?}",
                err
            );
        }
    }
```

  (Note: `test_sends_version_host_header` at lib.rs:602 uses `"acquire-1"` and `test_game_client_contract` uses `"test-game-1"` — both valid labels, unaffected.)
- [ ] Run: `cargo test -p brdgme_game_client` — the two behavioral tests FAIL before the implementation is in (the timeout test hangs/times-out on old code, the reset test gets an immediate error); with the implementation all pass. `cargo clippy -p brdgme_game_client --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
- [ ] Commit: `git add rust/lib/game_client/src/lib.rs` ; message: `fix(game_client): enforce per-attempt timeout ceiling, retry mid-request resets, validate version_name (ls F31 ls F33 ls F35, WP-07)`

---

### Task 3: parallelize fetch_game_data's post-Status requests; annotate the timing-sensitive test (ls F36 + ls F37, nits)

**Problem (restated):** `fetch_game_data` (lib.rs:212-302) issues Status → DataDocs → BasicStrategy → AdvancedStrategy → Rules strictly sequentially; the four after Status are independent, and the bot calls this once per turn through the interceptor where each call can pay cold-start latency. ls F37: `test_retry_on_connect_refused_then_success` (lib.rs:322-382) races a 15ms server spawn against a 20-40ms first backoff — the finding's own recommendation is "acceptable as-is", so it gets a comment, not a rewrite.

**Fix (re-derived, confirms F36):** `tokio::try_join!` the four post-Status requests (v2 path); the v1 path awaits only Rules. Fail-fast semantics (`try_join!` cancels the rest on first error) match the old sequential early-return. `Request::AdvancedStrategy` previously consumed `game` by move (lib.rs:274); it now clones like its siblings — same wire bytes.

**Files:**
- Modify: `rust/lib/game_client/src/lib.rs` (`fetch_game_data` body, one Cargo feature, one test comment), `rust/lib/game_client/Cargo.toml`

**Steps:**

- [ ] `rust/lib/game_client/Cargo.toml` line 17: `tokio = { version = "1", features = ["time"] }` → `tokio = { version = "1", features = ["macros", "time"] }` (`try_join!` lives behind `macros`; compile-time only, already in Cargo.lock via the dev-deps).
- [ ] Replace the section of `fetch_game_data` from the `let (data_docs, ...)` at lib.rs:244 through the `Rules` match ending at lib.rs:290 (the Status request, `player_render` extraction, and YAML conversion above it are unchanged from Task 1) with:

```rust
    let rules_fut = async {
        match request(client, uri, version_name, &Request::Rules).await? {
            Response::Rules { rules } => Ok(rules),
            _ => Err(GameClientError::UnexpectedResponse { request: "Rules" }),
        }
    };

    let (data_docs, basic_strategy, advanced_strategy, rules) = if interface_version >= 2 {
        // The four post-Status requests are independent; the bot fetches
        // this per turn, so run them concurrently (ls F36). try_join!
        // fails fast like the old sequential early-returns did.
        let dd_fut = async {
            match request(
                client,
                uri,
                version_name,
                &Request::DataDocs { game: game.clone() },
            )
            .await?
            {
                Response::DataDocs { data_docs } => Ok(data_docs),
                _ => Err(GameClientError::UnexpectedResponse {
                    request: "DataDocs",
                }),
            }
        };
        let bs_fut = async {
            match request(
                client,
                uri,
                version_name,
                &Request::BasicStrategy {
                    game: game.clone(),
                    player,
                },
            )
            .await?
            {
                Response::BasicStrategy { strategy } => Ok(strategy),
                _ => Err(GameClientError::UnexpectedResponse {
                    request: "BasicStrategy",
                }),
            }
        };
        let as_fut = async {
            match request(
                client,
                uri,
                version_name,
                &Request::AdvancedStrategy {
                    game: game.clone(),
                    player,
                },
            )
            .await?
            {
                Response::AdvancedStrategy { strategy } => Ok(strategy),
                _ => Err(GameClientError::UnexpectedResponse {
                    request: "AdvancedStrategy",
                }),
            }
        };
        tokio::try_join!(dd_fut, bs_fut, as_fut, rules_fut)?
    } else {
        let placeholder = "Not supported in game interface V1".to_string();
        let rules = rules_fut.await?;
        (placeholder.clone(), placeholder.clone(), placeholder, rules)
    };
```

  (The trailing `Ok(GameData { ... })` block at lib.rs:292-302 is unchanged.)
- [ ] Add a concurrency test with generous margins — 5 requests x 100ms delay: sequential would take ≥ 500ms, Status + parallel batch takes ~200ms; assert < 400ms:

```rust
    fn delayed_mock_game_server(delay: Duration) -> Router {
        Router::new().route(
            "/",
            post(move |req: Json<Request>| async move {
                tokio::time::sleep(delay).await;
                mock_game_response(req)
            }),
        )
    }
```

  This requires factoring the closure body of `mock_game_server` (lib.rs:641-690) into a shared `fn mock_game_response(Json(payload): Json<Request>) -> Json<Response>` containing the existing match, with `mock_game_server` and `delayed_mock_game_server` both routing to it. Then:

```rust
    #[tokio::test]
    async fn test_fetch_game_data_v2_parallelizes_followups() {
        // ls F36: the four post-Status requests must overlap. 5 x 100ms
        // sequential is >= 500ms; Status + a parallel batch is ~200ms.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, delayed_mock_game_server(Duration::from_millis(100)))
                .await
                .unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let start = std::time::Instant::now();
        let data = fetch_game_data(&client, &uri, "test-v2", "{}".to_string(), 0, 2)
            .await
            .expect("fetch_game_data failed");
        assert_eq!(data.data_docs, "V2 data docs");
        assert!(
            start.elapsed() < Duration::from_millis(400),
            "followup requests appear sequential: {:?}",
            start.elapsed()
        );
    }
```

- [ ] ls F37 — insert above the `drop(listener)` in `test_retry_on_connect_refused_then_success` (lib.rs:329):

```rust
        // Known race, accepted (ls F37): the server task binds ~15ms in while
        // the first jittered backoff is 20-40ms; with max_attempts=3 the
        // window for outright failure is negligible. If this ever flakes in
        // CI, bind the replacement listener on a second port first and point
        // the retry at that.
```

- [ ] Run: `cargo test -p brdgme_game_client` — all pass, including the pre-existing `test_fetch_game_data_v1_uses_placeholders` / `_v2_returns_real_content` / `_yaml_serialization` (lib.rs:701-737), which pin that the refactor changed no observable content. `cargo clippy -p brdgme_game_client --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
- [ ] Commit: `git add rust/lib/game_client/src/lib.rs rust/lib/game_client/Cargo.toml` ; message: `perf(game_client): parallelize fetch_game_data followup requests (ls F36 ls F37, WP-07)`

---

### Task 4: rand_bot — degenerate-spec panics, join separator, dependency trims (ls F43 + ls F41 + ls F40/dp F10 + ls F42)

**Problems (restated, all verified live in `rust/lib/rand_bot`):**

- ls F43 — `Spec::OneOf` with empty options panics (`options.choose(rng).unwrap()`, lib.rs:50); `Spec::Player` with empty `players` panics (lib.rs:84); `cli` unwraps the request-JSON parse (lib.rs:107) and the output serialization/write (lib.rs:111, 113). `Spec::Enum` already handles empties gracefully (lib.rs:45-48) — the fix mirrors it. Degenerate specs are exactly what the fuzz harness exists to find; the bot must emit an (invalid) command, not die.
- ls F41 — `spec_to_command` emits explicit `" "` tokens for `Spec::Space` (lib.rs:85), yet `commands()` joins with `" "` (lib.rs:93), double/triple-spacing every command, while tools/fuzz joins the same token stream with `""` (tools/fuzz/src/lib.rs:349). Joining with `""` respects the Space-token design; specs demonstrably carry explicit `Space` where separation matters (tools/fuzz has run this way).
- ls F40 = dp F10 — `chrono = { version = "0.4.45", features = ["serde"] }` (Cargo.toml:11) has zero references in the crate (verified: no `chrono` token in `src/`), and rand_bot is the workspace's only `chrono` consumer, so the whole chrono tree leaves the lock.
- ls F42 — `brdgme_cmd = { path = "../cmd" }` (Cargo.toml:9) takes default features = `http-server` = warp + tokio + sentry for a stdin/stdout bot that uses only `bot_cli::Request`. `game_client` already depends on `brdgme_cmd` with `default-features = false` and builds, proving the non-gated modules suffice.

**Files:**
- Modify: `rust/lib/rand_bot/src/lib.rs` (lines 49-51, 84, 93, 102-114 + new test module — NOT lines 98-101 or `main.rs`, which WP-06 owns), `rust/lib/rand_bot/Cargo.toml`

**Interfaces:** `spec_to_command`'s signature is unchanged (tools/fuzz calls it, tools/fuzz/src/lib.rs:349). Its behavior on empty `OneOf`/`Player` changes from panic to empty output — strictly more lenient for the only external caller.

**Steps:**

- [ ] ls F43, generation paths — mirror the `Enum` pattern. Replace the `OneOf` arm (lib.rs:49-51):

```rust
        command::Spec::OneOf(ref options) => options
            .choose(rng)
            .map(|o| spec_to_command(o, spec, players, rng))
            .unwrap_or_default(),
```

  and the `Player` arm (lib.rs:84):

```rust
        command::Spec::Player => players
            .choose(rng)
            .map(|p| vec![p.to_owned()])
            .unwrap_or_default(),
```

  (Empty output rather than the finding's "placeholder string" for `Player`: any output for a degenerate spec is an invalid command the game parser rejects; empty matches how `Enum` and now `OneOf` behave, and avoids inventing a magic name — see disposition table. `ctx` for `OneOf` children remains `spec`, the `OneOf` itself, as today.)
- [ ] ls F41 — line 93: `.join(" ")` → `.join("")` inside `commands()`. Space tokens now render exactly once (`"roll 2"` instead of `"roll   2"`), matching tools/fuzz's output shape.
- [ ] ls F43, `cli` boundary — replace the body (lib.rs:107-114):

```rust
    let request = serde_json::from_reader::<_, bot_cli::Request>(input)
        .expect("failed to parse bot request JSON from input");
    writeln!(
        output,
        "{}",
        serde_json::to_string(&commands(&request.command_spec, &request.players))
            .expect("failed to encode bot commands as JSON")
    )
    .expect("failed to write bot commands to output");
```

  Why `expect`, not graceful output: `cli`'s output type is `Vec<BotCommand>` — there is no in-band error channel, so emitting an empty list on a malformed *request* would silently mask a driver bug (unlike a degenerate *spec*, which is the fuzzer's legitimate discovery). A named panic at a process boundary is the WP-06/docs-CODING.md convention. Do NOT touch the comment block at lines 98-101 above the signature (WP-06 rewrites it).
- [ ] ls F40/dp F10 + ls F42 — `rust/lib/rand_bot/Cargo.toml` dependencies become:

```toml
[dependencies]
brdgme_cmd = { path = "../cmd", default-features = false }
brdgme_game = { path = "../game" }
rand = "0.10.2"
serde_json = "1.0.150"
```

  (chrono deleted; `default-features = false` added. Cargo.lock will drop the chrono/iana-time-zone subtree — expected, commit the lock change.)
- [ ] Add the test module (the crate currently has none) at the end of `rust/lib/rand_bot/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use brdgme_game::command::Spec;

    #[test]
    fn empty_oneof_yields_no_tokens_instead_of_panicking() {
        // ls F43: degenerate specs are the fuzzer's job to find; the bot
        // must produce an (invalid) command, not die.
        let mut rng = rand::rng();
        let spec = Spec::OneOf(vec![]);
        assert_eq!(
            Vec::<String>::new(),
            spec_to_command(&spec, &spec, &["a".to_string()], &mut rng)
        );
    }

    #[test]
    fn player_spec_with_no_players_yields_no_tokens() {
        let mut rng = rand::rng();
        let spec = Spec::Player;
        assert_eq!(
            Vec::<String>::new(),
            spec_to_command(&spec, &spec, &[], &mut rng)
        );
    }

    #[test]
    fn space_tokens_join_without_double_spaces() {
        // ls F41: Space emits an explicit " " token; joining with " " on top
        // produced "roll   2". The "" join matches tools/fuzz.
        let players = vec!["mick".to_string()];
        let spec = Spec::Chain(vec![
            Spec::Token("roll".to_string()),
            Spec::Space,
            Spec::Token("2".to_string()),
        ]);
        let bots = commands(&spec, &players);
        assert_eq!(vec!["roll 2".to_string()], bots[0].commands);
    }

    #[test]
    fn cli_writes_command_json_for_valid_request() {
        let req = bot_cli::Request {
            player: 0,
            player_state: "{}".to_string(),
            players: vec!["a".to_string()],
            command_spec: Spec::Token("go".to_string()),
            game_id: None,
        };
        let input = serde_json::to_vec(&req).unwrap();
        let mut out: Vec<u8> = vec![];
        cli(input.as_slice(), &mut out);
        let cmds: Vec<brdgme_game::bot::BotCommand> = serde_json::from_slice(&out).unwrap();
        assert_eq!(vec!["go".to_string()], cmds[0].commands);
    }
}
```

  (Red step: add the first two tests before the arm rewrites and run `cargo test -p brdgme_rand_bot` — both PANIC on the existing `unwrap()`s; the join test fails with `"roll   2"`. Then implement and re-run — all four pass. `BotCommand` derives Serialize/Deserialize with public `quality`/`commands` fields — lib/game/src/bot.rs:14-18.)
- [ ] Run: `cargo test -p brdgme_rand_bot`; `cargo clippy -p brdgme_rand_bot --all-targets -- -D warnings`; `cargo fmt --all -- --check`. Dependent builds: `cargo check -p brdgme_fuzz` (tools/fuzz, calls `spec_to_command`) and `cargo check -p brdgme_cmd --no-default-features` (proves the feature-trimmed dependency surface compiles independently of unification via other workspace members).
- [ ] Final package gate: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass (covers workspace-minus-web clippy/tests including every game bin, and web with ssr, against all four tasks' changes).
- [ ] Commit: `git add rust/lib/rand_bot/src/lib.rs rust/lib/rand_bot/Cargo.toml rust/Cargo.lock` ; message: `fix(rand_bot): no panics on degenerate specs, single-space joins, drop chrono and http-server pull (ls F40 ls F41 ls F42 ls F43 dp F10, WP-07)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| ls F31 no crate-enforced timeout | major | Default per-request timeout inside `send_with_retry`, "e.g. `request_builder.timeout(...)`, possibly on `RetryConfig`" | **Adjusted** (Task 2) | Goal confirmed (operator verified unbounded at controller.rs:230), mechanism overturned: `RequestBuilder::timeout` *replaces* the client-level timeout, silently overriding web's 10s and bot's 60s. Implemented as a `tokio::time::timeout` ceiling on `RetryConfig` (90s, above bot's deliberate 60s cold-start allowance) with min-semantics; body read bounded too; ceiling timeouts are retryable so the documented timeout-retry path now actually fires for the operator. |
| ls F32 anyhow in a library | minor | thiserror enum (Transport, HttpStatus, SystemError, UnexpectedResponse, Parse) | **Confirmed** (Task 1) | Variant list extended (`Timeout`, `InvalidVersionName`, `NoPlayerRender`, `StateJson`, `StateYaml`) to cover all live error sites. Full caller audit: 9 call sites across web/bot/operator; 8 are Display/`match`/`anyhow?`-compatible and compile unchanged; exactly one (web/src/game/mod.rs:114-124, `?` into `ExecuteCommandError`) needs a one-line `map_err` — a compile error, not a silent break. |
| ls F33 retry predicate too narrow | minor | "Consider also retrying `e.is_request()` ...; at minimum document the deliberate narrowness" | **Confirmed** (Task 2) | `is_request()` added (not just documented): safe because attempts are bounded, game-service requests are stateless (bot/src/main.rs:375-377), and received responses never reach the predicate. Locked by a raw-socket accept-then-drop test. |
| ls F35 version_name unvalidated in Host | nit | Validate DNS-label charset once in `request()`, clear error | **Confirmed** (Task 2) | Implemented in `request_with_config` (covers all public entry points incl. tests' path). All live names (DB `game_versions.name`, k8s object names) are DNS labels; no legitimate rejection. |
| ls F36 fetch_game_data 5 sequential trips | nit | `tokio::join!` the four post-Status requests, or accept and document | **Confirmed** (Task 3) | `try_join!` (fail-fast matches old early-return semantics). `AdvancedStrategy` now clones `game` instead of moving it — identical wire bytes. tokio `macros` feature added (already in lock). Timing test with 2.5x margin. |
| ls F37 timing-sensitive retry test | nit | "Acceptable as-is; bind the replacement listener first on a second port if flakiness appears" | **Confirmed as accept** (Task 3) | No behavioral change, per the finding's own recommendation; the race and its escape hatch are now documented in a comment at the race site. |
| ls F40 chrono unused | minor | Delete the dependency | **Confirmed** (Task 4) | Zero source references (re-verified live); rand_bot is the sole workspace consumer, so the lock drops the whole chrono subtree. |
| dp F10 chrono vs time split | minor | Same finding as ls F40 (per w6-botops-deps.md mapping: "chrono in rand_bot vs time everywhere else", rand-bot-time group) | **Confirmed** (Task 4) | Discharged by the same deletion — no second datetime lib remains. |
| ls F41 join separator vs tools/fuzz | minor | Join with `""` and share the join — "have tools/fuzz call RandBot's `commands`" | **Adjusted** (Task 4) | First half confirmed (`""` join, matching the Space-token design; tools/fuzz proves specs carry explicit Spaces). Sharing half deferred: tools/fuzz is WP-63's path and out of this package's scope — recorded as a coordination note for WP-63, which can now call `commands()` for an identical result. |
| ls F42 http-server stack pulled unused | minor | `default-features = false` | **Confirmed** (Task 4) | `bot_cli`/`api` are not feature-gated (lib/cmd/src/lib.rs:7-14); `game_client` already builds against `brdgme_cmd` with `default-features = false`, proving the surface. Verified additionally with `cargo check -p brdgme_cmd --no-default-features`. |
| ls F43 panics on degenerate specs | minor | Mirror `Enum`'s `unwrap_or_default` for `OneOf`; "for `Player`, fall back to a placeholder string"; (lib.rs:107 JSON parse also listed) | **Adjusted** (Task 4) | `OneOf` confirmed as recommended. `Player` adjusted: empty vec instead of a placeholder string — any degenerate-spec output is an equally invalid command, and consistency with `Enum`/`OneOf` beats a magic name. `cli`'s JSON-parse/write unwraps adjusted to `expect` with real messages rather than graceful output: `Vec<BotCommand>` has no in-band error channel, and swallowing a malformed *request* would mask driver bugs (WP-06 process-boundary precedent). The `Int min>max` panic at lib.rs:32-34 is a further degenerate-spec panic NOT named by any finding — left untouched for scope discipline; flag for a future sweep if wanted. |

## Cross-package coordination

- **WP-06 (lib cmd, already specced):** its Task 4 edits `rand_bot/src/main.rs` (deletes the `extern crate` line, ls F44) and rewrites the comment at `rand_bot/src/lib.rs:98-101` as a doc comment on `cli` (ls F45), and keeps `bot_cli::Request` in `brdgme_cmd`. This package's lib.rs hunks (49-51, 84, 93, 107-113, appended test module) are disjoint; land in either order and rebase. If WP-06 lands first, this package's Cargo `default-features = false` still works — WP-06 does not gate `bot_cli`.
- **WP-63 (tools/fuzz):** after ls F41, `brdgme_rand_bot`'s private `commands()` produces exactly what tools/fuzz's hand-rolled `spec_to_command(...).join("")` produces; WP-63 may deduplicate by making `commands` public and calling it (optional rider noted for its author).
- **WP-70 (serde_yaml, D-21):** game_client's `serde_yaml = "0.9"` line and `json_to_yaml` body are untouched here; the new `StateYaml(#[from] serde_yaml::Error)` variant is the one place WP-70's migration must re-point its error type.
- **WP-62 (operator):** the operator's bare `Client::new()` is now bounded by the crate ceiling (90s/attempt); WP-62 may still add connect/client timeouts for parity with web/bot, but nothing requires it. Its `format!("{e:#}")` logging keeps working (Display-based) — the alternate-flag chain rendering it got from anyhow is replaced by self-contained Display messages on `GameClientError`.
- **WP-64 (workspace-deps):** the four manifest edits here (drop anyhow/chrono, add thiserror/tokio-macros, `default-features = false`) become root-level one-liners when WP-64 migrates; sequence-independent.
- **web/src/game/mod.rs one-liner:** inside `execute_command`, which WP-38/WP-40 (blocked packages) will later touch; the `.map_err(anyhow::Error::from)` is a trivial rebase for whichever lands second.
