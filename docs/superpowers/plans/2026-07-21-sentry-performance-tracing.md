# Sentry Performance & Distributed Tracing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn on Sentry performance tracing, web vitals, DB query spans, and distributed tracing across web -> game services -> bot, replacing the application observability lost by dropping Grafana Cloud.

**Architecture:** Extend the existing error-only Sentry (sentry/sentry-tower/sentry-tracing 0.48 in `rust/web`, `@sentry/browser` 10.65 in the SSR snippet). Phase 1 enables web HTTP transactions + browser tracing/web-vitals + selective `#[tracing::instrument]` DB spans + Sentry-side release health/uptime/alerts. Phase 2 propagates traces web -> game services (reqwest middleware out, manual warp filter in via shared `lib/cmd`). Phase 3 propagates traces through the bot/NATS path with explicit care around JetStream ack semantics.

**Tech Stack:** Rust (sentry 0.48, sentry-tower, sentry-tracing, axum, warp, sqlx, async-nats, reqwest), Leptos/WASM (@sentry/browser 10.65), Sentry SaaS (free Developer plan).

## Global Constraints

- Official / battle-hardened integrations only; no new telemetry dependencies beyond what Sentry provides (DB spans use `#[tracing::instrument]`, NOT `sqlx-tracing`).
- Stay within the free Developer plan: 5,000 errors/month, 5M spans/month, 50 replays/month. Spans are the only category this work adds volume to.
- `traces_sample_rate` / `tracesSampleRate` = 0.1 everywhere (uniform; parent-based inheritance for continued traces).
- `sendDefaultPii` / `send_default_pii` stays `false`; the existing `beforeSend` scrubbing is unchanged.
- DSN-unset = disabled no-op (dev/Tilt/CI unaffected) - the established convention from the 2026-07-15 spec. Every Sentry call site must remain a no-op without a bound client.
- Session Replay, Profiling, Seer AI, Sentry Logs/App-Metrics are OUT (deferred per spec).
- Phase 3 must NOT change bot JetStream WorkQueue ack/retry behaviour (`ack_wait` 5m, `max_deliver` 3).
- Rust verification gate before any commit (from AGENTS.md): `cargo fmt --all -- --check`; `cargo clippy -p web --all-targets --features ssr -- -D warnings`; `cargo clippy --workspace --exclude web --all-targets -- -D warnings`. DB-backed tests fail in a plain local run (known, pre-existing - not a regression).

## File Structure

- `rust/web/src/router.rs` - enable_transaction + matched-path transaction-naming middleware (Phase 1).
- `rust/web/src/main.rs` - `traces_sample_rate` in `init_sentry` (Phase 1).
- `rust/web/src/app.rs` - browser `browserTracingIntegration` + `tracesSampleRate` in the SSR init snippet (Phase 1).
- `rust/web/src/db.rs` (+ `auth/server.rs`, `proposals.rs`) - `#[tracing::instrument]` on hot query fns (Phase 1).
- `rust/web/Cargo.toml` - sentry `reqwest` feature + reqwest `middleware` feature (Phase 2).
- `rust/web/src/state.rs` - Sentry reqwest middleware on `http_client` (Phase 2).
- `rust/lib/cmd/src/http.rs` - warp Sentry filter + `sentry::init` for game services (Phase 2).
- `rust/lib/cmd/Cargo.toml` - sentry dependency for the game server (Phase 2).
- `rust/web/src/game/mod.rs` - NATS header injection in `publish_bot_turns` (Phase 3).
- `rust/bot/src/main.rs` - `sentry::init` + sentry_tracing layer + consumer trace continuation (Phase 3).
- `rust/bot/Cargo.toml` - sentry + sentry-tracing dependencies (Phase 3).

---

# Phase 1 - Web + browser + DB + Sentry-side features

### Task 1: Enable web-server HTTP transactions

**Files:**
- Modify: `rust/web/src/router.rs:193`
- Modify: `rust/web/src/main.rs:234-242` (init_sentry ClientOptions)

**Interfaces:**
- Produces: a Sentry transaction per HTTP request, continuing inbound `sentry-trace`/`baggage` headers; later tasks (DB spans, reqwest propagation) nest under / continue from it.

- [ ] **Step 1: Enable transactions on the HTTP layer**

In `rust/web/src/router.rs`, change line 193 from:

```rust
        .layer(SentryHttpLayer::new())
```

to:

```rust
        .layer(SentryHttpLayer::new().enable_transaction())
```

- [ ] **Step 2: Set the traces sample rate**

In `rust/web/src/main.rs`, in `init_sentry` (the `sentry::ClientOptions` block, ~line 234), add `traces_sample_rate`:

```rust
        sentry::ClientOptions {
            release,
            send_default_pii: false,
            traces_sample_rate: 0.1,
            ..Default::default()
        },
```

- [ ] **Step 3: Verify it builds and the no-op path is intact**

Run: `cargo build -p web --features ssr`
Expected: compiles cleanly.

Run: `cargo test -p web --features ssr ssr_pages` (the in-process SSR tests run with no DSN bound)
Expected: PASS - confirms the layers remain a no-op without a client (DB-backed tests may fail locally; that is the known pre-existing condition).

- [ ] **Step 4: Commit**

```bash
git add rust/web/src/router.rs rust/web/src/main.rs
git commit -m "feat(web): enable Sentry HTTP transactions (traces_sample_rate 0.1)"
```

### Task 2: Low-cardinality transaction names (matched path)

**Files:**
- Modify: `rust/web/src/router.rs` (add middleware + wire it)

**Interfaces:**
- Consumes: the transaction started by `SentryHttpLayer::enable_transaction()` (Task 1).
- Produces: transaction names like `GET /admin/games/{id}/export` instead of per-ID URIs.

- [ ] **Step 1: Write the failing test**

Add to `rust/web/src/router.rs` (or a `#[cfg(test)]` module therein) a test that the naming helper formats `METHOD route`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn transaction_name_format() {
        assert_eq!(
            super::sentry_transaction_name("GET", "/admin/games/{id}/export"),
            "GET /admin/games/{id}/export"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p web --features ssr router::tests::transaction_name_format`
Expected: FAIL - `sentry_transaction_name` not defined.

- [ ] **Step 3: Add the helper and middleware**

In `rust/web/src/router.rs`, add near the other middleware fns:

```rust
fn sentry_transaction_name(method: &str, route: &str) -> String {
    format!("{} {}", method, route)
}

/// Sets the Sentry transaction name from the matched route (low cardinality),
/// mirroring the `MatchedPath` source `make_root_span` uses. No-op without a
/// bound Sentry client (dev/Tilt/CI). Must run inside the SentryHttpLayer
/// transaction and after routing has populated `MatchedPath`.
async fn set_sentry_transaction_name(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    if let Some(route) = request.extensions().get::<MatchedPath>().map(MatchedPath::as_str) {
        let name = sentry_transaction_name(request.method().as_str(), route);
        sentry::configure_scope(|scope| scope.set_transaction(name));
    }
    next.run(request).await
}
```

Wire it into the router. Place it just inside the Sentry layers so it runs within the transaction and after `MatchedPath` is set - i.e. add it immediately BEFORE the `.layer(SentryHttpLayer::new().enable_transaction())` line (axum applies `.layer()` outermost-first, so this sits inside the Sentry transaction):

```rust
        .layer(middleware::from_fn(set_sentry_transaction_name))
        .layer(SentryHttpLayer::new().enable_transaction())
        .layer(NewSentryLayer::<Request<axum::body::Body>>::new_from_top())
```

- [ ] **Step 4: Run the test + build**

Run: `cargo test -p web --features ssr router::tests`
Expected: PASS.

Run: `cargo build -p web --features ssr`
Expected: compiles. (If `MatchedPath` is not yet populated at this layer position, move the middleware one layer inward and re-verify; the SSR test plus a manual DSN check in Task 6 confirms the name lands.)

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/router.rs
git commit -m "feat(web): name Sentry transactions by matched route (low cardinality)"
```

### Task 3: Browser tracing + Core Web Vitals

**Files:**
- Modify: `rust/web/src/app.rs:54-67` (sentry_init_snippet)

**Interfaces:**
- Consumes: `window.Sentry.browserTracingIntegration` (already in the `@sentry/browser` bundle exposed by `js/sentry.js`).
- Produces: frontend transactions + web vitals; same-origin propagation to the web server (default `tracePropagationTargets`).

- [ ] **Step 1: Add browserTracingIntegration + tracesSampleRate to the init snippet**

In `rust/web/src/app.rs`, replace `sentry_init_snippet` (the comment + `format!`, ~lines 54-67). The old comment ("No `tracesSampleRate` key ... disables the tracing integration") is now stale - update it:

```rust
fn sentry_init_snippet(dsn: &str, release: Option<&str>) -> String {
    let release_field = release
        .map(|r| format!(r#","release":"{}""#, js_string_escape(r)))
        .unwrap_or_default();
    // Performance tracing ON (2026-07-21): browserTracingIntegration captures
    // frontend transactions + Core Web Vitals; tracesSampleRate 0.1 keeps span
    // volume within the free Developer plan's 5M/month. Same-origin propagation
    // to the web server is the default (the browser does not call game services
    // directly). See docs/superpowers/specs/2026-07-21-sentry-performance-tracing-design.md.
    format!(
        r#"window.Sentry.init({{"dsn":"{}","integrations":[window.SentryWasmIntegration(),window.Sentry.browserTracingIntegration()],"sendDefaultPii":false,"tracesSampleRate":0.1{},"beforeSend":{}}});"#,
        js_string_escape(dsn),
        release_field,
        SENTRY_BEFORE_SEND_JS,
    )
}
```

- [ ] **Step 2: Verify the snippet renders and the bundle exposes the integration**

Run: `cargo build -p web --features ssr`
Expected: compiles.

Confirm `browserTracingIntegration` is exported by the bundle:
Run: `grep -c "browserTracingIntegration" rust/web/public/sentry.js`
Expected: a non-zero count (the bundled `@sentry/browser` exposes it; `window.Sentry.browserTracingIntegration` resolves at runtime).

- [ ] **Step 3: Commit**

```bash
git add rust/web/src/app.rs
git commit -m "feat(web): enable Sentry browser tracing + web vitals (tracesSampleRate 0.1)"
```

### Task 4: DB query spans on hot paths (`#[tracing::instrument]`)

**Files:**
- Modify: `rust/web/src/db.rs`
- Modify: `rust/web/src/auth/server.rs`, `rust/web/src/proposals.rs` (the hot auth/proposal query paths)

**Interfaces:**
- Consumes: the active request transaction (Tasks 1-2); the existing `sentry_tracing::layer()` in `main.rs` captures these spans as children.
- Produces: `db`-op child spans with low-cardinality identifiers on the hot queries.

- [ ] **Step 1: Add the instrument attribute to the hot query functions**

In `rust/web/src/db.rs`, add `#[tracing::instrument(skip(pool), fields(...))]` to the hot/important functions. `skip(pool)` keeps the pool out of span data; record only low-cardinality IDs. Examples (apply the same pattern to the full hot list below):

```rust
#[tracing::instrument(skip(pool), fields(user_id = %id))]
pub async fn get_user(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
```

```rust
#[tracing::instrument(skip(pool), fields(game_id = %id))]
pub async fn find_game(pool: &PgPool, id: Uuid) -> Result<Option<crate::models::game::Game>> {
```

```rust
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn find_bot_turns(pool: &PgPool, game_id: Uuid) -> Result<Vec<BotTurn>> {
```

Hot list to instrument (same pattern; for `&mut PgConnection` fns use `skip(conn)`):
- `get_user_by_email` (fields: none - email is PII, skip it; `skip(pool)`)
- `get_user`, `find_game`, `find_game_extended`, `find_bot_turns`
- `find_active_game_summaries`, `create_game_with_users_tx`, `update_game_command_success`
- `undo_game`, `concede_game`, `mark_game_read`
- In `auth/server.rs`: the login-verification and session queries.
- In `proposals.rs`: `create_proposal`, `respond_proposal`, `start_proposal_early`, `get_proposal`.

For functions taking `conn: &mut sqlx::PgConnection`, use `#[tracing::instrument(skip(conn), fields(...))]`.

- [ ] **Step 2: Verify it builds and clippy is clean**

Run: `cargo build -p web --features ssr`
Expected: compiles.

Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: no warnings (instrument macros can trip `clippy::needless_lifetimes`-style lints only rarely; fix if flagged).

- [ ] **Step 3: Commit**

```bash
git add rust/web/src/db.rs rust/web/src/auth/server.rs rust/web/src/proposals.rs
git commit -m "feat(web): Sentry DB spans via tracing::instrument on hot query paths"
```

### Task 5: Sentry-side config - release health, uptime, alerts (manual)

**Files:** none (Sentry SaaS configuration)

- [ ] **Step 1: Confirm release health is active**

In the Sentry UI, open the web-server and web-frontend projects > Releases. Confirm releases appear (the `SENTRY_RELEASE` string is already set on both SDKs) and that crash-free session/user rate is charting after a deploy. No code change; release health keys off the existing `release`.

- [ ] **Step 2: Create the free uptime monitor**

Sentry UI > Alerts > Create Alert > Uptime Monitor. Target the public site URL (`https://beta.brdg.me`). 1 monitor is included free. Alert on failure.

- [ ] **Step 3: Create the three metric alerts**

Sentry UI > Alerts > Create Alert (Metric Alert), one each:
- Error-rate spike: when the error count for a project exceeds a baseline threshold over 5m.
- Crash-free-rate drop: when crash-free session rate falls below 99% over 1h (front-end project).
- p95 latency regression: when p95 transaction duration for key routes (`/`, game pages) exceeds a threshold over 10m.

- [ ] **Step 4: Record the config in the spec status**

Update the Status line of `docs/superpowers/specs/2026-07-21-sentry-performance-tracing-design.md` to note Phase 1 + Sentry-side config complete with the date.

- [ ] **Step 5: Commit the status update**

```bash
git add docs/superpowers/specs/2026-07-21-sentry-performance-tracing-design.md
git commit -m "docs: mark Sentry tracing Phase 1 + alerts/uptime/release-health complete"
```

### Task 6: Phase 1 verification (with a real DSN)

- [ ] **Step 1: Deploy to a DSN-bearing environment and exercise it**

With `SENTRY_DSN_SERVER`/`SENTRY_DSN_WEB` set (beta), load a few pages and trigger a server fn. In Sentry > Traces, confirm: a transaction named by matched route (e.g. `GET /`), child `db` spans from Task 4, and frontend transactions + Web Vitals (LCP/INP/CLS) from the browser.

- [ ] **Step 2: Confirm span volume is within quota**

Sentry > Settings > Usage. Confirm span usage is trending well under 5M/month at the 0.1 sample rate after a day of beta traffic.

---

# Phase 2 - Game-service distributed tracing

### Task 7: Propagate traces on outbound game calls (reqwest middleware)

**Files:**
- Modify: `rust/web/Cargo.toml` (sentry `reqwest` feature; reqwest `middleware` feature)
- Modify: `rust/web/src/state.rs` (where `http_client` is built)

**Interfaces:**
- Consumes: the active web transaction (Phase 1).
- Produces: `sentry-trace`/`baggage` headers on `game_client` reqwest calls, which the game-service filter (Task 8) continues.

- [ ] **Step 1: Enable the required features**

In `rust/web/Cargo.toml`, add the `reqwest` feature to the `sentry` dependency and the `middleware` feature to `reqwest`:

```toml
sentry = { version = "0.48", optional = true, features = ["reqwest"] }
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls", "middleware"], optional = true }
```

(Verify the exact feature name for Sentry's reqwest integration at 0.48 - it is `reqwest`, exposing `sentry::integrations::reqwest::SentryMiddleware`.)

- [ ] **Step 2: Add the middleware where the Client is built**

Locate the `reqwest::Client` construction that populates `AppState.http_client` (search `rust/web/src` for `reqwest::Client::builder`). Add the Sentry middleware:

```rust
let http_client = reqwest::Client::builder()
    .add_middleware(sentry::integrations::reqwest::SentryMiddleware::new())
    /* ...existing config (timeouts, etc.)... */
    .build()
    .expect("reqwest client");
```

- [ ] **Step 3: Build + clippy**

Run: `cargo build -p web --features ssr`
Expected: compiles.

Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add rust/web/Cargo.toml rust/Cargo.lock rust/web/src/state.rs
git commit -m "feat(web): propagate Sentry trace headers on outbound game-client calls"
```

### Task 8: Continue traces in the game services (shared warp filter)

**Files:**
- Modify: `rust/lib/cmd/src/http.rs`
- Modify: `rust/lib/cmd/Cargo.toml` (add sentry)

**Interfaces:**
- Consumes: `sentry-trace`/`baggage` headers from Task 7.
- Produces: a game-service transaction per request, parented to the web trace; covers all ~25 games via the shared `serve()`.

- [ ] **Step 1: Add the sentry dependency to lib/cmd**

In `rust/lib/cmd/Cargo.toml`, add (optional, behind a feature so games without a DSN stay no-op):

```toml
sentry = { version = "0.48", optional = true }
```

and a `sentry` feature that enables it. Add `sentry` to each game binary's features (or make it non-optional in `lib/cmd` if all games should carry it - decision: non-optional is simpler since all games share `serve()`; the no-op-when-unset convention keeps dev/CI clean).

- [ ] **Step 2: Initialise Sentry + add the trace-continuation filter in `serve()`**

In `rust/lib/cmd/src/http.rs`, replace `serve` to (a) init Sentry from a `SENTRY_DSN_SERVER` env (unset = disabled no-op, same convention as web), and (b) wrap the handler in a filter that continues the inbound trace. Verify the exact warp filter-composition and `TransactionContext::continue_from_headers` API against the pinned sentry/warp versions while implementing:

```rust
pub async fn serve<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    addr: impl Into<SocketAddr>,
) {
    env_logger::init();
    let _guard = std::env::var("SENTRY_DSN_SERVER").ok().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: std::env::var("SENTRY_RELEASE").ok().map(std::borrow::Cow::Owned),
                send_default_pii: false,
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });

    let handler = warp::post()
        .and(warp::header::headers_cloned())
        .and(warp::body::json())
        .map(|headers: warp::http::HeaderMap, req: Request| {
            let ctx = sentry::TransactionContext::continue_from_headers(
                headers.iter().map(|(k, v)| {
                    (k.to_string(), v.to_str().unwrap_or_default().to_string())
                }),
            );
            let transaction = sentry::start_transaction(ctx);
            sentry::configure_scope(|scope| scope.set_span(Some(transaction.clone().into())));
            let mut g: GameRequester<G> = requester::gamer::new();
            let reply = warp::reply::json(&g.request(&req).unwrap());
            transaction.finish();
            reply
        });

    let shutdown = async {
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    warp::serve(handler)
        .bind(addr.into())
        .await
        .graceful(shutdown)
        .run()
        .await
}
```

- [ ] **Step 3: Build the workspace (games included)**

Run: `cargo build -p no-thanks-2` (a representative game binary) and `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
Expected: compiles, clippy clean.

- [ ] **Step 4: Verify end-to-end (with DSNs)**

With DSNs set on web + a game, trigger a web action that calls a game service. In Sentry > Traces, confirm the game transaction is a child of the web trace (same trace id).

- [ ] **Step 5: Commit**

```bash
git add rust/lib/cmd/src/http.rs rust/lib/cmd/Cargo.toml rust/Cargo.lock
git commit -m "feat(games): continue Sentry traces in shared warp game server"
```

---

# Phase 3 - Bot / NATS tracing (care: preserve ack semantics)

### Task 9: Inject trace headers into `bot.turn` (web side)

**Files:**
- Modify: `rust/web/src/game/mod.rs:192-236` (publish_bot_turns)

**Interfaces:**
- Consumes: the active web transaction.
- Produces: `sentry-trace`/`baggage` NATS message headers the bot continues (Task 10).

- [ ] **Step 1: Build NATS headers from the current transaction and publish with headers**

In `rust/web/src/game/mod.rs`, in `publish_bot_turns`, replace the `jetstream.publish(...)` call with `publish_with_headers`, carrying the current transaction's headers:

```rust
        let mut headers = async_nats::HeaderMap::new();
        sentry::configure_scope(|scope| {
            if let Some(span) = scope.get_span() {
                for (k, v) in span.iter_headers() {
                    headers.insert(k.as_str(), v.parse().unwrap_or_default());
                }
            }
        });
        match jetstream
            .publish_with_headers(crate::nats::SUBJECT_TURN, headers, payload.into())
            .await
        {
            Ok(ack) => {
                if let Err(e) = ack.await {
                    tracing::warn!(%game_id, "bot.turn publish not acked: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!(%game_id, "Failed to publish bot.turn: {}", e);
            }
        }
```

(Verify `scope.get_span()` / `span.iter_headers()` names against sentry 0.48; the intent is to copy `sentry-trace` + `baggage` from the active transaction into the NATS headers. When no client is bound, `configure_scope` is a no-op and `headers` is empty - publish behaviour is unchanged.)

- [ ] **Step 2: Build + clippy**

Run: `cargo build -p web --features ssr` and `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add rust/web/src/game/mod.rs
git commit -m "feat(web): propagate Sentry trace headers on bot.turn NATS publish"
```

### Task 10: Continue the trace in the bot consumer

**Files:**
- Modify: `rust/bot/src/main.rs:681` (tracing init) + the `bot.turn` consumer loop (~743+)
- Modify: `rust/bot/Cargo.toml` (add sentry + sentry-tracing)

**Interfaces:**
- Consumes: `sentry-trace`/`baggage` NATS headers from Task 9.
- Produces: a bot-turn transaction parented to the web trace; ack/retry behaviour UNCHANGED.

- [ ] **Step 1: Add dependencies**

In `rust/bot/Cargo.toml`:

```toml
sentry = { version = "0.48", optional = true }
sentry-tracing = { version = "0.48", optional = true }
```

behind a `sentry` feature.

- [ ] **Step 2: Init Sentry + a sentry_tracing layer in the bot**

In `rust/bot/src/main.rs`, replace `tracing_subscriber::fmt::init();` (~line 681) with a registry that adds the sentry layer, plus a `sentry::init` guard held for the process lifetime:

```rust
    let _sentry_guard = std::env::var("SENTRY_DSN_SERVER").ok().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: std::env::var("SENTRY_RELEASE").ok().map(std::borrow::Cow::Owned),
                send_default_pii: false,
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .init();
```

- [ ] **Step 3: Continue the trace per bot.turn message, finishing before ack**

In the consumer loop (`while let Some(message) = messages.next().await`, ~743), wrap the turn processing in a transaction continued from the message headers, and finish it BEFORE the existing `message.ack()` calls. The ack calls and their error handling stay exactly as they are:

```rust
        let ctx = sentry::TransactionContext::continue_from_headers(
            message.headers.iter().flat_map(|h| {
                h.iter().map(|(k, v)| (k.to_string(), v.to_string()))
            }),
        );
        let transaction = sentry::start_transaction(ctx);
        let result = {
            let _guard = transaction.clone();
            sentry::configure_scope(|scope| scope.set_span(Some(transaction.clone().into())));
            /* ...existing turn-processing call(s)... */
        };
        transaction.finish();
        /* ...existing message.ack() logic, unchanged... */
```

(Verify `message.headers` access against the async-nats version; the intent is to read the NATS headers map. The transaction MUST finish before ack so a Sentry flush cannot delay or disturb the ack/retry path.)

- [ ] **Step 4: Build + clippy**

Run: `cargo build -p bot` and `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Verify ack semantics are unchanged**

Run the bot's existing NATS eventing tests:
Run: `cargo test -p web --features ssr nats_bot_eventing` (and any bot tests)
Expected: PASS - no regression in publish/ack/redelivery behaviour.

Then, with DSNs set, trigger a bot turn and confirm in Sentry that the bot-turn transaction is a child of the web trace, and that bot turns still process/ack normally (no extra redeliveries).

- [ ] **Step 6: Commit**

```bash
git add rust/bot/src/main.rs rust/bot/Cargo.toml rust/Cargo.lock
git commit -m "feat(bot): continue Sentry traces from bot.turn headers (ack semantics preserved)"
```

---

## Final verification

- [ ] Run the full Rust gate: `cargo fmt --all -- --check`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`, `cargo clippy --workspace --exclude web --all-targets -- -D warnings`. Expected: all clean.
- [ ] Confirm a full end-to-end trace in Sentry: browser -> web -> game, and browser -> web -> NATS -> bot, all sharing one trace id.
- [ ] Confirm span usage stays under the free 5M/month after a week (Sentry > Settings > Usage).
- [ ] Update the spec Status line to mark all three phases complete with the date.
