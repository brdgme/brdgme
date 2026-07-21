# Sentry Performance & Distributed Tracing - Design

Date: 2026-07-21
Status: Complete 2026-07-21. All three phases implemented (web/browser/DB, game services, bot/NATS). Extends the error-only Sentry foundation from
`docs/superpowers/specs/2026-07-15-wasm-prod-errors-design.md` (WS3), which
deliberately omitted `tracesSampleRate` to protect the free-tier error quota.
This work turns performance/tracing ON, now that the project is on the
free Developer plan with 5M spans/month included (spans are abundant and
cheap; the binding constraints there are errors and replays, not spans).

## Problem

brdgme's Sentry is error-only. Both SDK inits deliberately omit a traces
sample rate, so Sentry captures errors/panics/breadcrumbs but produces no
transactions, no spans, no web vitals, and no distributed traces. Meanwhile
the project is dropping Grafana Cloud / Alloy (see the companion
`2026-07-21-alloy-grafana-removal-design.md`), which removes the only other
application-metrics and (disabled) tracing path. The result would be a
cluster with good error visibility but no way to answer "is the app fast and
healthy?" - no request latency, no slow-route or slow-query visibility, no
frontend web vitals, no deploy-health signal, and no alerting.

Sentry already ships a full application-monitoring platform; brdgme simply
hasn't switched it on. This spec enables the high-value, low-cost subset of
it using only official / battle-hardened integrations, in a way that is easy
to maintain and adds minimal boilerplate.

## Goals

1. Request-level performance visibility for the web server (per-route
   latency, slow handlers).
2. Frontend performance + Core Web Vitals for the Leptos/WASM client.
3. Database query timing on the hot paths.
4. Distributed traces that follow a request from the browser through the web
   server into the game microservices, and through the bot/NATS path.
5. Deploy-health + alerting to replace the Grafana Cloud alerts being
   removed (release health, uptime, metric alerts).
6. Stay within the free Developer plan's quotas; no surprise billing.

## Research findings (verified 2026-07-21)

- **Plan/quota:** The free Developer plan includes 5,000 errors/month AND
  5M spans/month (1 seat, 30-day retention). Spans are billed only beyond
  quota at ~$0.0000016/span. A single traced request is ~20-50 spans, so 5M
  spans is ~100K+ traced requests before any overage. For beta traffic,
  tracing fits inside the included quota with a modest sample rate. The
  binding free-tier limits are errors (5K) and replays (50) - not spans.
- **Axum integration is already installed and official.** `sentry-tower`
  (0.48) IS Sentry's official Axum integration (Axum is tower-based; there is
  no separate sentry-axum). `rust/web/src/router.rs` already wires
  `NewSentryLayer::new_from_top()` + `SentryHttpLayer::new()` in the correct
  Axum order. The catch: `SentryHttpLayer::new()` only attaches request
  metadata to captured errors. Per the sentry-tower docs, a transaction per
  request requires `.enable_transaction()`, which also continues the trace
  from inbound `sentry-trace`/`baggage` headers (distributed-tracing inbound
  for free). And `sentry::init` must set `traces_sample_rate` (currently
  `..Default::default()` = 0.0 = no transactions ever sent).
- **Browser integration is already bundled.** `rust/web/js/sentry.js` sets
  `window.Sentry = Sentry` (the full `@sentry/browser` 10.65 namespace) and
  `window.SentryWasmIntegration`. So `window.Sentry.browserTracingIntegration`
  is already available at runtime - enabling browser tracing/web vitals is a
  change to the SSR init snippet in `rust/web/src/app.rs`, with NO bundle
  change and NO new dependency.
- **`sentry_tracing` already captures tracing spans.** The web tracing
  subscriber already runs `sentry_tracing::layer()`. Per its docs, tracing
  spans at/above Info are recorded as Sentry spans (child spans of the active
  transaction), and `#[tracing::instrument]` creates a span/transaction
  automatically. So DB query spans via `#[tracing::instrument]` need no new
  layer - only the instrument attributes plus an active request transaction
  to nest under.
- **DB instrumentation choice:** `#[tracing::instrument]` (battle-hardened,
  no new dependency, full control, more boilerplate) vs `sqlx-tracing`
  (wrap the pool once, every query auto-spanned, less boilerplate, but a
  far less popular/battle-hardened crate). Decision: `#[tracing::instrument]`
  applied SELECTIVELY to hot/important paths (see below), revisiting
  `sqlx-tracing` only if coverage proves too sparse (documented fallback).
- **Game services use `warp`, not tower/axum.** `rust/lib/cmd/src/http.rs`
  serves games via `warp::serve` + `env_logger`. So `sentry-tower`'s layers
  do NOT apply to games; that phase needs a manual warp filter that continues
  the inbound trace and starts a transaction. The saving grace: all ~25 game
  binaries share `lib/cmd`'s `serve()`, so the change lands in one place.
- **Trace topology (verified):** the browser talks to the WEB server
  (Leptos server fns over same-origin HTTP + `/ws`); games are called
  SERVER-SIDE via `lib/game_client` (reqwest, setting a
  `{version}.games.internal` HOST header to route through the gateway/KEDA
  interceptor); bot turns flow web -> NATS (`bot.turn`) -> bot -> NATS
  (`bot.command`) -> web consumer. So traces are linear:
  browser -> web -> game (reqwest), and browser -> web -> NATS -> bot.
  The browser does NOT call games directly, so browser trace propagation
  only needs same-origin (the default `tracePropagationTargets`).
- **Outbound reqwest propagation:** Sentry's `reqwest` integration
  (`sentry::integrations::reqwest::SentryMiddleware`, behind the `reqwest`
  feature) injects `sentry-trace`/`baggage` from the current span into
  outgoing requests. web's `AppState.http_client` is a `reqwest::Client`
  (`rust/web/src/state.rs:11`); adding the middleware there propagates traces
  into game-service calls. (reqwest's `middleware` feature must be enabled.)
- **NATS propagation is manual.** async-nats publish can carry headers
  (`publish_with_headers`). The web `publish_bot_turns`
  (`rust/web/src/game/mod.rs:192`) injects `sentry-trace`/`baggage` from the
  current transaction; the bot consumer continues the trace via
  `TransactionContext::continue_from_headers` and wraps the turn handler in a
  transaction. sentry-tracing also supports a `sentry.trace` span field for
  continuing a trace at a service boundary using only the tracing API.

## Decision

One spec, one phased implementation plan. Official/battle-hardened
integrations only; minimal boilerplate; stay within the free Developer plan.

### Feature scope

In scope (high value, fits free quota):
- Server performance tracing (HTTP transactions + spans).
- Browser tracing + Core Web Vitals.
- DB query spans on hot paths (`#[tracing::instrument]`).
- Distributed tracing into game services and through the bot/NATS path.
- Release health (release string already set via `SENTRY_RELEASE`).
- The free uptime monitor (1 included).
- Metric alerts: error-rate spike, crash-free-rate drop after a release,
  p95 latency regression on key routes.

Deferred (cost/quota or low current value - documented, not forgotten):
- Session Replay (only 50/month free, then ~$0.003/replay; privacy-sensitive
  given the deliberate `sendDefaultPii: false` stance).
- Profiling (PAYG-only: continuous $0.0315/hr, UI $0.25/hr).
- Seer AI ($40/active contributor/month, billed separately).
- Sentry Logs / Application Metrics products (5GB free each, but brdgme uses
  stdout logs + is dropping the metrics pipeline; revisit if needs change).

### Sampling

- Web server: `traces_sample_rate = 0.1` (uniform). Low beta traffic keeps
  this well within 5M spans; raise later if more fidelity is wanted.
- Browser: `tracesSampleRate = 0.1` to match.
- Game services: parent-based inheritance - a continued trace inherits the
  caller's sampling decision, so games don't independently over- or
  under-sample continued traces. Game-originated transactions (if any) use a
  low rate.
- Bot: modest uniform rate on the bot-turn transaction; bot turns are
  discrete units of work and a natural transaction boundary.

### Transaction naming (cardinality control)

`SentryHttpLayer` names transactions by raw request URI, so routes with path
params (e.g. `/admin/games/{id}/export`, game/invite routes) would create a
unique transaction name per ID - useless aggregation. Fix: a small Axum
middleware sets the transaction name from the `MatchedPath` (the same
low-cardinality source `make_root_span` already uses in `router.rs`) via
`sentry::configure_scope(|s| s.set_transaction(route))`. This avoids
relying on a version-specific matched-path feature flag.

## Architecture

### Phase 1 - web + browser + DB + Sentry-side features (executable soon)

- **Web server transactions:** `router.rs:193` `SentryHttpLayer::new()` ->
  `SentryHttpLayer::new().enable_transaction()`; `main.rs` `init_sentry`
  adds `traces_sample_rate: 0.1` to `ClientOptions`. Add the matched-path
  transaction-naming middleware.
- **Browser tracing + web vitals:** `app.rs` `sentry_init_snippet` adds
  `window.Sentry.browserTracingIntegration()` to `integrations` and a
  `tracesSampleRate` field. Same-origin propagation to web is the default;
  no `tracePropagationTargets` override needed (browser doesn't call games).
- **DB query spans:** `#[tracing::instrument(skip_all)]` on the hot/important
  query functions in `rust/web/src/db.rs` and the auth/proposals paths -
  e.g. `get_user_by_email`, `get_user`, `find_game`, `find_game_extended`,
  `find_bot_turns`, `find_active_game_summaries`, `create_game_with_users_tx`,
  `update_game_command_success`, `undo_game`, `concede_game`. Selective, not
  exhaustive; `skip(pool)`/`skip_all` keeps span payloads small. Captured by
  the existing `sentry_tracing` layer as children of the request transaction.
- **Sentry-side config (no code):** enable release health (release already
  set), create the free uptime monitor against the public URL, and add metric
  alerts (error-rate spike, crash-free-rate drop, p95 latency on key routes).

### Phase 2 - game-service distributed tracing

- **Outbound (web -> game):** enable the sentry `reqwest` feature + reqwest
  `middleware` feature; add `SentryMiddleware` to the `AppState.http_client`
  builder so `game_client` calls carry `sentry-trace`/`baggage`.
- **Inbound (game services):** in shared `rust/lib/cmd/src/http.rs`, add a
  warp filter that (a) reads `sentry-trace`/`baggage` request headers,
  (b) starts a transaction via `TransactionContext::continue_from_headers`,
  (c) binds it to the scope for the handler, (d) finishes it on completion.
  Add `sentry::init` (DSN from env, unset = disabled no-op, same convention
  as web) to the shared `serve()` so all ~25 games get it from one change.
  Replace/augment `env_logger` so game logs correlate (optional; games may
  keep env_logger and just add Sentry).

### Phase 3 - bot/NATS tracing (with care)

- **Outbound (web -> NATS):** `publish_bot_turns` builds NATS headers from
  the current transaction (`sentry-trace` + `baggage`, via the transaction's
  `iter_headers()`) and uses `publish_with_headers` instead of `publish`.
- **Inbound (bot):** add `sentry::init` + a `sentry_tracing` layer to the bot
  (switch `bot/src/main.rs:681` from `tracing_subscriber::fmt::init()` to a
  registry with the fmt + sentry layers, mirroring web). In the `bot.turn`
  consumer loop, read the message headers, continue the trace via
  `TransactionContext::continue_from_headers`, and wrap the turn-processing
  in a transaction that finishes before the message is acked.
- **Care points (explicit):**
  - Do NOT disturb the JetStream WorkQueue ack/retry semantics
    (`ack_wait` 5m, `max_deliver` 3): the Sentry transaction must finish and
    Sentry errors must be captured WITHOUT changing when/how the message is
    acked or retried.
  - Keep Sentry isolated from game logic: a Sentry failure/panic must not
    fail the turn or the ack. Sentry calls are best-effort no-ops when no
    client is bound (dev/CI), matching the web convention.
  - Sample modestly; bot turns can be numerous relative to beta traffic.

## Out of scope (rejected / deferred)

- Session Replay, Profiling, Seer AI, Sentry Logs/App-Metrics (see Feature
  scope rationale above).
- Migrating game services from warp to axum just to reuse `sentry-tower` -
  disproportionate; the manual warp filter is contained in `lib/cmd`.
- `sqlx-tracing` as the primary DB-span mechanism (less battle-hardened);
  kept only as a documented fallback if `#[tracing::instrument]` coverage is
  judged too sparse after Phase 1 lands.
- 100% sampling anywhere (quota hygiene; raise later with data).
- Changing the error-only `beforeSend` scrubbing or `sendDefaultPii: false`
  posture established in the 2026-07-15 spec.

## Success criteria

1. A request to a web route produces a Sentry transaction named by its
   matched route (low cardinality), with child spans for instrumented DB
   queries.
2. The Leptos frontend reports Core Web Vitals (LCP/INP/CLS) and frontend
   transactions to Sentry.
3. A trace initiated in the browser is visible end-to-end through the web
   server into a game-service transaction (Phase 2) and through a bot turn
   (Phase 3).
4. Release health shows crash-free rate per release; the uptime monitor and
   the three metric alerts are configured and firing on test conditions.
5. Span volume stays within the free Developer plan's 5M/month at the chosen
   sample rates (verified in Sentry usage stats after a week).
6. Bot ack/retry behaviour is unchanged (no regression in bot-turn
   processing or JetStream redelivery) after Phase 3.
