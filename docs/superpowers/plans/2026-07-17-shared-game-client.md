# Shared Game Service Client Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the game service HTTP client (KEDA interceptor `Host` header, retry/backoff, response handling) into one shared crate used by web, bot, and operator - fixing the bot's current 404 failures in the process.

**Architecture:** New workspace library crate `rust/lib/game_client` (package `brdgme_game_client`) containing the code currently in `rust/web/src/game/client.rs`. Web re-exports it under its existing `crate::game::client` path; bot and operator replace their hand-rolled request functions with it. The `Host: {version_name}.games.internal` convention required by the KEDA HTTP interceptor then lives in exactly one place.

**Tech Stack:** Rust (edition 2024), reqwest 0.13 (rustls), axum 0.8 + tokio (dev-only, for mock servers in tests), existing `brdgme_cmd::api` request/response types.

## Background (why)

- All 39 `game_versions.uri` rows point at the KEDA HTTP interceptor
  (`http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080`),
  which routes purely on the `Host` header (`{version_name}.games.internal`,
  matching each `HTTPScaledObject`'s `spec.hosts`).
- Web sets the header (`rust/web/src/game/client.rs`, commit `c9c5d94`) and
  the operator has its own copy (`rust/operator/src/controller.rs:55`), but
  the bot's `call_game_service` (`rust/bot/src/main.rs:440`) does not - every
  bot Status/Play call gets 404 from the interceptor and games get stuck
  waiting on the bot (observed in prod bot logs 2026-07-17).
- This host-header-to-interceptor approach is the officially documented KEDA
  HTTP add-on pattern for in-cluster callers; the fix is to centralise the
  knowledge, not to change the infrastructure.

## Global Constraints

- Rust edition 2024; workspace at `rust/Cargo.toml`, resolver = "2".
- Library crates live under `rust/lib/<name>` with package name `brdgme_<name>` (e.g. `brdgme_cmd`).
- Canonical verification commands (all with `SQLX_OFFLINE=true`, per `docs/DEV.md`):
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `cargo test --workspace --exclude web`
  - `cargo test -p web --features ssr` (needs dev Postgres for the ~41 DB tests)
- `web` has no default features - it always needs `--features ssr` (plain `cargo check --workspace` fails by design).
- The game interface JSON contract (`docs/ARCHITECTURE.md`) is stable and must not change.
- **Execution mode: single batch, single push.** Work through the tasks in order in one session, committing per task locally, and push to `master` only once after Task 5 completes. This triggers one CI pipeline and one image build/deploy cycle for all three services together. There is no urgency requiring a staged rollout of the bot fix.
- Do not edit applied sqlx migrations. No schema changes are needed in this plan (`game_versions.name` already exists).
- Format with `cargo fmt --all` before each commit.

---

### Task 1: Create the `brdgme_game_client` crate

**Files:**
- Create: `rust/lib/game_client/Cargo.toml`
- Create: `rust/lib/game_client/src/lib.rs` (moved from `rust/web/src/game/client.rs`)
- Modify: `rust/Cargo.toml` (workspace members)

**Interfaces:**
- Consumes: `brdgme_cmd::api::{Request, Response, PubRender, PlayerRender}`, `brdgme_game::command::Spec`.
- Produces (public API, used by Tasks 2-4; identical signatures to today's `web::game::client`):
  - `pub async fn request(client: &reqwest::Client, uri: &str, version_name: &str, request: &Request) -> anyhow::Result<Response>`
  - `pub async fn render(client: &reqwest::Client, uri: &str, version_name: &str, game: String, player: Option<usize>) -> anyhow::Result<RenderResponse>`
  - `pub async fn pub_render(...)`, `pub async fn player_render(...)` as in the current file
  - `pub struct RenderResponse { pub render: String, pub state: String, pub command_spec: Option<CommandSpec> }`

- [ ] **Step 1: Scaffold the crate with the moved code**

Create `rust/lib/game_client/Cargo.toml`:

```toml
[package]
name = "brdgme_game_client"
version = "0.1.0"
publish = false
authors = ["Michael Alexander <beefsack@gmail.com>"]
edition = "2024"

[dependencies]
anyhow = "1.0.103"
brdgme_cmd = { path = "../cmd", default-features = false }
brdgme_game = { path = "../game" }
rand = "0.10.2"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
serde_json = "1.0.150"

[dev-dependencies]
axum = "0.8.9"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "net"] }
```

Copy `rust/web/src/game/client.rs` verbatim to `rust/lib/game_client/src/lib.rs` (implementation and its `#[cfg(test)] mod tests` included), then add this crate-level doc comment at the top of the file:

```rust
//! Shared HTTP client for calling game services through the KEDA HTTP
//! interceptor. All in-cluster callers (web, bot, operator) MUST use this
//! crate: the interceptor routes purely on the Host header
//! (`{version_name}.games.internal`), which this client sets on every
//! request. Calling the interceptor without that header returns 404.
```

Add `"lib/game_client",` to the `members` list in `rust/Cargo.toml` (after `"lib/game"`, keeping the existing ordering style).

- [ ] **Step 2: Run the moved tests to verify the crate stands alone**

Run: `cd rust && cargo test -p brdgme_game_client`
Expected: PASS (all tests moved across: retry, backoff, contract tests)

- [ ] **Step 3: Add a Host-header regression test**

Append to the `tests` module in `rust/lib/game_client/src/lib.rs`:

```rust
#[tokio::test]
async fn test_sends_version_host_header() {
    use axum::http::HeaderMap;
    // Echo the received Host header back in pub_state so the assertion can
    // see exactly what the client sent.
    let app = Router::new().route(
        "/",
        post(|headers: HeaderMap, Json(_payload): Json<Request>| async move {
            let host = headers
                .get(axum::http::header::HOST)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            Json(Response::PubRender {
                render: PubRender {
                    pub_state: host,
                    render: String::new(),
                },
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
    let resp = pub_render(&client, &uri, "acquire-1", "g".to_string())
        .await
        .expect("request failed");
    assert_eq!(
        resp.state, "acquire-1.games.internal",
        "client must send Host {{version_name}}.games.internal for KEDA interceptor routing"
    );
}
```

- [ ] **Step 4: Run the new test**

Run: `cd rust && cargo test -p brdgme_game_client test_sends_version_host_header`
Expected: PASS (the moved code already sets the header; this test pins the contract so it can never silently regress again)

- [ ] **Step 5: Write a failing test for non-2xx status errors**

Today a non-2xx response (e.g. the interceptor's plain-text `404 Not Found`) surfaces as an opaque JSON parse error (`error parsing response: Not Found`). The client should report the HTTP status instead. Extend the existing `test_no_retry_on_http_error_response` test - after the existing `assert!(resp.is_err(), ...)` line, add:

```rust
        let err = format!("{:#}", resp.unwrap_err());
        assert!(
            err.contains("500"),
            "error must include the HTTP status, got: {err}"
        );
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cd rust && cargo test -p brdgme_game_client test_no_retry_on_http_error_response`
Expected: FAIL - the error is currently `error parsing response: boom`, which does not contain `500`

- [ ] **Step 7: Implement the status check**

In `request_with_config` in `rust/lib/game_client/src/lib.rs`, replace:

```rust
    let res = send_with_retry(client, uri, version_name, request, config).await?;
    let body = res.text().await.context("error reading response body")?;
```

with:

```rust
    let res = send_with_retry(client, uri, version_name, request, config).await?;
    let status = res.status();
    let body = res.text().await.context("error reading response body")?;
    if !status.is_success() {
        return Err(anyhow!("game service returned {status}: {body}"));
    }
```

- [ ] **Step 8: Run the crate tests**

Run: `cd rust && cargo test -p brdgme_game_client`
Expected: PASS (all tests, including both new ones)

- [ ] **Step 9: Commit**

```bash
cd rust && cargo fmt --all
git add rust/Cargo.toml rust/Cargo.lock rust/lib/game_client
git commit -m "feat: extract shared brdgme_game_client crate for interceptor-aware game service calls"
```

---

### Task 2: Web adopts the shared crate

**Files:**
- Delete: `rust/web/src/game/client.rs`
- Modify: `rust/web/src/game/mod.rs:1-2` (module declaration)
- Modify: `rust/web/Cargo.toml` (dependency + ssr feature)

**Interfaces:**
- Consumes: `brdgme_game_client` public API from Task 1.
- Produces: `crate::game::client::{request, render, pub_render, player_render, RenderResponse}` keeps resolving for all existing web call sites (`server_fns.rs`, `game/mod.rs`) - no call-site changes.

- [ ] **Step 1: Swap the module for a re-export**

In `rust/web/src/game/mod.rs`, replace:

```rust
#[cfg(feature = "ssr")]
pub mod client;
```

with:

```rust
#[cfg(feature = "ssr")]
pub use brdgme_game_client as client;
```

Delete `rust/web/src/game/client.rs` (its tests moved to the crate in Task 1).

In the doc comment on `spawn_mock_game_service` (`rust/web/src/game/mod.rs:408`), update the reference `mirrors the pattern in `game::client::tests`` to `mirrors the pattern in `brdgme_game_client`'s tests`.

- [ ] **Step 2: Wire the dependency**

In `rust/web/Cargo.toml`:
- Next to the other `brdgme_*` path deps (around line 48), add:
  ```toml
  brdgme_game_client = { path = "../lib/game_client", optional = true }
  ```
- In the `ssr` feature list (after `"dep:brdgme_game"`), add:
  ```toml
      "dep:brdgme_game_client",
  ```

- [ ] **Step 3: Verify web builds and tests pass**

Run:
```bash
cd rust
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
SQLX_OFFLINE=true cargo test -p web --features ssr
```
Expected: clippy clean; tests PASS (DB-backed tests need the dev Postgres from `docs/DEV.md`; if it is unavailable, the ~41 DB tests fail with connection timeouts - that failure mode is pre-existing and not caused by this change, but prefer running with the DB up)

- [ ] **Step 4: Commit**

```bash
cd rust && cargo fmt --all
git add rust/web rust/Cargo.lock
git commit -m "refactor: web uses shared brdgme_game_client"
```

---

### Task 3: Bot adopts the shared crate (fixes the 404 bug)

**Files:**
- Modify: `rust/bot/Cargo.toml` (dependency)
- Modify: `rust/bot/src/main.rs` (query, AppState, load_bot_context, validation call; delete local `call_game_service`)

**Interfaces:**
- Consumes: `brdgme_game_client::request` from Task 1.
- Produces: nothing new - behavioral fix only. Bot game-service calls now send `Host: {version_name}.games.internal`.

- [ ] **Step 1: Add the dependency**

In `rust/bot/Cargo.toml`, next to the other `brdgme_*` deps (around line 21):

```toml
brdgme_game_client = { path = "../lib/game_client" }
```

- [ ] **Step 2: Fetch the version name in the trigger query**

In `rust/bot/src/main.rs` (~line 67), change the SELECT to also return the game version name:

```sql
SELECT g.game_state, gv.uri, gv.name as version_name, gv.rules, gt.name as game_name, gb.name as bot_name, gp.is_turn, gp.id as game_player_id
```

and extract it next to the other row reads (~line 94):

```rust
    let version_name: String = row.try_get("version_name").context("version_name")?;
```

- [ ] **Step 3: Add a dedicated game-service HTTP client**

The bot's existing `state.http` has a 300s timeout sized for LLM calls, and the old `call_game_service` applied a 10s per-request timeout - too tight for KEDA scale-from-zero cold starts. Add a second client sized for game services.

In `AppState` (~line 20), after `http: reqwest::Client,` add:

```rust
    /// Client for game service calls: shorter timeout than the LLM client,
    /// but generous enough for KEDA scale-from-zero cold starts.
    game_http: reqwest::Client,
```

In `main` (~line 659), after the existing `http` client is built:

```rust
    let game_http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to build game service HTTP client")?;
```

and add `game_http,` to the `AppState { ... }` initializer.

- [ ] **Step 4: Route all game-service calls through the shared client**

Thread `version_name` into `load_bot_context` - new signature:

```rust
async fn load_bot_context(
    state: &AppState,
    game_service_uri: &str,
    version_name: &str,
    game_id: uuid::Uuid,
    player_position: i32,
    game_player_id: uuid::Uuid,
    game_state: String,
    names: &[String],
) -> Result<BotContext> {
```

Inside it, replace the `call_game_service(...)` Status call with:

```rust
    let status_resp = brdgme_game_client::request(
        &state.game_http,
        game_service_uri,
        version_name,
        &Request::Status {
            game: game_state.clone(),
        },
    )
    .await
    .context("Game service Status call failed")?;
```

Update both `load_bot_context` call sites (~lines 147 and 242) to pass `&version_name` after `&game_service_uri`.

Replace the validation call (~line 260):

```rust
        let validate_result = brdgme_game_client::request(
            &state.game_http,
            &game_service_uri,
            &version_name,
            &Request::Play {
                player: req.player_position as usize,
                game: bot_ctx.game_state.clone(),
                command: command.clone(),
                names: names.clone(),
            },
        )
        .await;
```

The surrounding `match validate_result` arms are compatible as-is: `Ok(Response::Play)` / `Ok(Response::UserError)` behave identically, and `Response::SystemError` (previously surfacing via the `Ok(_)` arm) now arrives as `Err(e)` with the service's message - still recorded as `error_body`.

Delete the now-unused local `async fn call_game_service` (~lines 440-460).

- [ ] **Step 5: Verify the workspace builds and tests pass**

Run:
```bash
cd rust
SQLX_OFFLINE=true cargo clippy -p bot --all-targets -- -D warnings
SQLX_OFFLINE=true cargo test --workspace --exclude web
```
Expected: clippy clean, tests PASS. (The Host-header behavior itself is pinned by `test_sends_version_host_header` in the crate; the bot has no unit tests around `handle_bot_turn`, so correctness here is type-checked wiring plus the prod verification section below.)

- [ ] **Step 6: Commit**

```bash
cd rust && cargo fmt --all
git add rust/bot rust/Cargo.lock
git commit -m "fix: bot sends interceptor Host header via shared game client

Bot game-service calls were 404ing at the KEDA HTTP interceptor because
they lacked the {version}.games.internal Host header, leaving games stuck
waiting on bot turns."
```

---

### Task 4: Operator adopts the shared crate

**Files:**
- Modify: `rust/operator/Cargo.toml` (dependency)
- Modify: `rust/operator/src/controller.rs:49-67` (`game_service_request`)

**Interfaces:**
- Consumes: `brdgme_game_client::request` from Task 1.
- Produces: `game_service_request` keeps its existing signature `(client, uri, name, request) -> Result<Response, Error>` so its call sites (~lines 129, 142) are untouched.

- [ ] **Step 1: Add the dependency**

In `rust/operator/Cargo.toml`, next to `brdgme_cmd` (~line 18):

```toml
brdgme_game_client = { path = "../lib/game_client" }
```

- [ ] **Step 2: Delegate to the shared client**

Replace the body of `game_service_request` in `rust/operator/src/controller.rs`:

```rust
async fn game_service_request(
    client: &reqwest::Client,
    uri: &str,
    name: &str,
    request: &Request,
) -> Result<Response, Error> {
    brdgme_game_client::request(client, uri, name, request)
        .await
        .map_err(|e| Error::GameService(format!("{e:#}")))
}
```

(`Response::SystemError` was already mapped to `Error::GameService`; the shared client folds it into the `Err` path with the same message. The operator also gains the crate's bounded retry on connect failures/timeouts, which is strictly more robust for reconcile-time calls against scaled-to-zero services.)

If this leaves the operator's `Error` enum with a now-unused reqwest error variant (compiler/clippy will say so), delete that variant and its `#[from]` impl in the same commit.

- [ ] **Step 3: Verify**

Run:
```bash
cd rust
SQLX_OFFLINE=true cargo clippy -p brdgme-operator --all-targets -- -D warnings
SQLX_OFFLINE=true cargo test --workspace --exclude web
```
Expected: clippy clean, tests PASS

- [ ] **Step 4: Commit**

```bash
cd rust && cargo fmt --all
git add rust/operator rust/Cargo.lock
git commit -m "refactor: operator uses shared brdgme_game_client"
```

---

### Task 5: Document the convention

**Files:**
- Modify: `docs/ARCHITECTURE.md:110-112` (Game Interface Contract intro)

**Interfaces:** none (docs only).

- [ ] **Step 1: Record the client rule in the architecture doc**

In `docs/ARCHITECTURE.md`, directly after the paragraph ending "This contract is stable and must not change." (line 112), add:

```markdown
All in-cluster callers reach game services through the KEDA HTTP
interceptor, which routes on a `Host: {version_name}.games.internal`
header. The shared crate `rust/lib/game_client` (`brdgme_game_client`)
owns this convention plus retry/backoff; web, bot, and the operator all
call game services through it. Never hand-roll a game service HTTP call -
a request without the Host header gets a 404 from the interceptor.
```

- [ ] **Step 2: Commit**

```bash
git add docs/ARCHITECTURE.md
git commit -m "docs: record shared game client and interceptor Host convention"
```

---

## Prod verification (after deploy)

Deployment is the usual flow: CI builds images, bump `brdgme-config/prod/kustomization.yaml` (bot, web, operator images + kustomize ref), ArgoCD syncs.

1. Stuck bot turns are NATS-redelivered, so previously stuck games should self-heal once the new bot image is live. Watch for the errors disappearing:
   `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml logs -n brdgme deploy/bot --tail=50` - no new `Game service returned 404` errors.
2. Play a bot game on beta.brdg.me against a scaled-to-zero game version (any `*-1` version) and confirm the bot moves; first move may take a few seconds (cold start), well within the new 60s timeout.
3. Confirm web and operator behave unchanged: game pages render, and `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml logs -n brdgme deploy/operator --tail=20` shows normal reconciles.
