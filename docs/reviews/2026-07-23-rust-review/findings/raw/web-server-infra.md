# Raw findings — web crate server infrastructure

Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/web/`
Reviewer unit: web-server-infra (READ-ONLY; no builds/tests run).
Scope files read IN FULL: `src/main.rs`, `src/router.rs`, `src/state.rs`,
`src/config.rs`, `src/nats.rs`, `src/error.rs`, `src/websocket.rs`,
`src/websocket_client.rs`, `src/bin/import_game.rs`, `Cargo.toml`.
Skimmed: `src/auth/blocked_domains.rs` (head/tail + usage sites only).
Also read for grounding (referenced by in-scope files / scrutiny list):
`src/auth/session.rs` (session layer config), `k8s/base/web/deployment.yaml`
(env wiring), `rust/web/.env.template`, `rust/Cargo.toml` (workspace),
`rust/Cargo.lock` (single grep for rustls provider crates only).
leptos-use `use_websocket` v0.19.0 upstream source fetched to verify `open()`
semantics (see finding below).

Line numbers are per snapshot.

---

### Session cookies lack the Secure flag in production (SECURE_COOKIE never set)
- severity: major
- category: correctness
- location: web/src/auth/session.rs:32-36 (config read), k8s/base/web/deployment.yaml:40-44 (env block, no SECURE_COOKIE), web/.env.template (does not document it)
- finding: `create_session_layer` reads `SECURE_COOKIE` and defaults to
  `with_secure(false)`. A repo-wide grep finds `SECURE_COOKIE` in exactly two
  places: the `session.rs` read and a completed plan doc
  (`docs/superpowers/plans/2026-07-02-07-pre-cutover-fixes.md`). It is NOT set
  in any k8s manifest (base/dev/prod), not in any of the three `secretRef`s
  plausibly (postgres-config / email-config / database-encryption-key), and not
  documented in `.env.template`. So production pods run with
  `with_secure(false)`: the session cookie is set over HTTPS but will also be
  transmitted by browsers over any plaintext `http://` request to the domain
  (ssl-strip / any non-redirected HTTP endpoint), exposing the session token.
  SameSite=Lax and HttpOnly (tower-sessions default) are fine; 30-day
  OnInactivity expiry is reasonable.
- recommendation: Set `SECURE_COOKIE=true` in `k8s/base/web/deployment.yaml`
  env (and document it in `web/.env.template`). Consider defaulting to secure
  unless an explicit `INSECURE_COOKIE`/`DEV` opt-out is set, so the safe
  behavior is the default.

### Bot command consumer task is unsupervised — silent permanent bot outage on exit/panic
- severity: major
- category: correctness
- location: web/src/main.rs:55-74
- finding: `run_bot_command_consumer` is spawned once. If it returns `Err`,
  one log line is emitted and the task exits forever — bots stop moving in
  every game until the pod restarts. If the task *panics*, the JoinHandle is
  dropped and nothing restarts it either. Meanwhile `/healthz` keeps returning
  OK (it is deliberately DB-independent and doesn't cover the consumer), so
  k8s sees a healthy pod; the only signal is a single log line. In-flight
  messages are safe (un-acked → redelivered after `ack_wait`), but no new
  processing ever resumes.
- recommendation: Supervise: wrap the consumer in a restart loop with backoff
  (log + sleep + re-enter), or `tokio::select!` the consumer future against
  `axum::serve` so consumer death aborts the process and lets k8s restart it,
  or add consumer liveness to a deeper health check. At minimum emit a
  metric/Sentry event on exit so the outage is alertable.

### JetStream: messages that exhaust max_deliver=3 strand silently (no DLQ, no advisory handling, no alert)
- severity: minor
- category: correctness
- location: web/src/nats.rs:63-94
- finding: Both durable pull consumers use `AckPolicy::Explicit`,
  `ack_wait = 5 min`, `max_deliver = 3` on a `WorkQueue`-retention stream.
  A message that fails delivery 3 times (consumer crash loops, persistent
  handler error) is never redelivered; with WorkQueue retention it is only
  deleted on ack, so it sits in the stream indefinitely — the bot simply never
  moves in that game again, with no signal anywhere (NATS emits a
  MAX_DELIVERIES advisory, but nothing subscribes to it). Uncertainty: the
  consumer implementation (`web::game::run_bot_command_consumer`, out of
  scope) may `term()` poison messages or otherwise bound this; if it does,
  downgrade to nit. Severity could be major if no such handling exists —
  worth a cross-check by the unit reviewing `game/`.
- recommendation: Either `term()` + compensate (e.g. re-publish with backoff,
  surface "bot stuck" in the game UI) on permanent failure in the consumer, or
  add a DLQ subject / max-deliveries advisory listener with alerting. Consider
  a stream `max_age` so stranded messages can't accumulate forever.

### get_or_create_stream/consumer never reconcile config drift
- severity: minor
- category: correctness
- location: web/src/nats.rs:52-94
- finding: `get_or_create_stream` and `get_or_create_consumer` return the
  existing object untouched when it already exists — changing `ack_wait`,
  `max_deliver`, `retention`, or subjects in this file is silently a no-op
  against an existing NATS deployment. The values in code can therefore
  diverge from what the server actually enforces, with no warning at startup.
  (Certain for JetStream server semantics; the exact async-nats code path —
  get-then-create vs create-then-ignore-conflict — unverified, but neither
  updates config.)
- recommendation: After get-or-create, compare the returned
  `stream.info().config` / `consumer.info().config` against the desired
  values and log a warning (or fail startup) on mismatch; document that
  consumer config changes require manual consumer deletion/recreation.

### /ws WebSocket endpoint: no authentication, site-wide wildcard fan-out to every connection
- severity: minor
- category: correctness
- location: web/src/websocket.rs:82-87 (handler, no session extraction), web/src/websocket.rs:112-125 (wildcard subs), web/src/router.rs:142 (route)
- finding: `ws_handler` takes no `Session` and does no auth check. Every
  connection — including anonymous ones — creates two core-NATS subscriptions
  on `game.>` and `proposal.>` and receives EVERY game/proposal update signal
  site-wide. Impact: (a) information disclosure — an unauthenticated client
  gets a live firehose of all game/proposal UUIDs and their activity timing;
  the payloads are skinny (just UUIDs) and game pages presumably enforce
  authorization on fetch, so this is an enumeration/activity-metadata leak
  rather than a data leak; (b) scalability — outbound WS traffic is
  O(connections × total site update rate), and every client reactively
  processes every signal even for games it isn't viewing (dedupe happens
  client-side via the (id, seq) memo, but the message still crosses the wire
  and the reactive graph). The per-game subjects (`game.<uuid>`) already exist
  on the publish side, so per-connection filtering is feasible if the client
  declared its interests.
- recommendation: If the firehose is an accepted design (plausible at current
  scale), document the decision near `ws_handler`. Otherwise: accept an
  initial subscribe message listing game/proposal IDs (or derive from the
  session's active games) and subscribe per-connection to those subjects
  instead of wildcards; require a valid session for `/ws` if anonymous
  activity metadata matters. Revisit the O(N×M) fan-out before user counts
  grow.

### Client calls `open()` on every visibilitychange/online — leptos-use tears down and re-creates healthy sockets (verified upstream)
- severity: minor
- category: correctness
- location: web/src/websocket_client.rs:70-85
- finding: The `visibilitychange` (→ visible) and `online` listeners call
  leptos-use's `open()` unconditionally. Verified against leptos-use v0.19.0
  source (`use_websocket.rs`): `open()` → `connect()` closes any existing
  socket and creates a brand-new `WebSocket` *unconditionally* — there is no
  ready-state guard. Worse, closing the old socket fires its `onclose`, whose
  handler schedules an auto-reconnect timer (3s) because the stored socket's
  state is momentarily non-OPEN; when that timer fires it calls `connect()`
  again, replacing the just-established socket a second time. Net effect:
  every tab refocus causes ~1–2 gratuitous WS reconnects (plus server-side
  connection churn: two new NATS subscriptions per reconnect), and a brief
  signal-loss window each time. Runtime behavior not directly observed —
  inferred from upstream source; treat the "second reconnect" as probable but
  unverified. The local `bump_game_update` on action success already covers
  the "WS was down" refetch case these handlers are presumably protecting.
- recommendation: Gate the calls on `ready_state` (already returned by
  `use_websocket_with_options` and currently destructured as `ready_state: _`)
  — only call `open()` when state is `Closed`. That keeps the recovery intent
  without churning healthy connections.

### Graceful shutdown does not cover WS connections or background tasks
- severity: minor
- category: quality
- location: web/src/main.rs:104-110 (with_graceful_shutdown), web/src/websocket.rs:108 (detached socket tasks), web/src/main.rs:55-80 (consumer/sweep tasks)
- finding: `axum::serve(...).with_graceful_shutdown` stops accepting and
  drains in-flight HTTP requests, but axum/hyper does not track upgraded
  WebSocket connections — they live in detached `tokio::spawn`ed tasks and are
  dropped when the runtime shuts down after `serve` returns. The bot consumer
  and email sweep tasks likewise get no shutdown signal. Practical impact is
  low: WS clients reconnect (`ReconnectLimit::Infinite`), and an aborted
  bot.command is redelivered via `ack_wait` — but the shutdown contract is
  worth stating: during deploys every connected client hard-drops, and any
  sweep mid-send is abandoned.
- recommendation: Acceptable as-is for a beta; if desired, track WS tasks with
  a `CancellationToken`/TaskTracker and close sockets with a proper close
  frame on shutdown, and pass the token into the consumer/sweep spawns.

### ack_wait=5min may be shorter than consumer processing → duplicate delivery
- severity: minor
- category: correctness
- location: web/src/nats.rs:63
- finding: `ack_wait = 5 * 60s`. If the `bot.command` consumer's processing
  (bot HTTP call to the bot service + DB writes + possible retries) ever
  exceeds 5 minutes without an ack or an in-progress extension (`nak` with
  delay / `in_progress()`), JetStream redelivers to another replica and the
  command runs twice. Bot commands are likely idempotent-ish (stale-state
  conflict handling exists per `BotCommandEvent::attempt` docs), so impact is
  bounded. Uncertain: consumer's ack cadence is out of scope (web/src/game/).
- recommendation: Cross-check with the game/consumer unit: confirm the
  consumer acks promptly or sends progress pings; otherwise raise `ack_wait`
  or have the consumer call `message.in_progress()` during long work.

### No rustls CryptoProvider installed in web's main — runtime panic risk if both providers ever enter web's graph
- severity: minor
- category: correctness
- location: web/src/main.rs:5 (no provider install), web/Cargo.toml:28 (sqlx runtime-tokio-rustls), :46 (resend rustls-tls), :70 (reqwest rustls)
- finding: The project rule (AGENTS.md/CODING.md): binaries using crates that
  read the process-default rustls `CryptoProvider` must install one in `main`,
  because the workspace enables both `aws-lc-rs` and `ring` (both are present
  in the workspace `Cargo.lock` — confirmed by grep). `web`'s `main` installs
  no provider. Web's TLS-client surface: `reqwest` (rustls), `resend-rs`
  (rustls-tls), `sqlx` (runtime-tokio-rustls). If web's dependency graph ever
  enables *both* rustls providers, `rustls::ClientConfig::builder()`-style
  calls panic at first TLS handshake ("no process-level CryptoProvider
  available..."). Today this apparently doesn't fire (production resend email
  works, so exactly one provider is presumably enabled in web's graph — likely
  `ring` is pulled only by `operator`'s `kube`, and/or each rustls consumer
  uses `builder_with_provider`), but nothing in this crate guards it and a
  dependency bump could flip it. Uncertain — flagged per the review brief's
  explicit request to check this risk; resolving it definitively requires
  dependency-graph analysis (`cargo tree -f "{p} {f}" -i rustls`), which is
  out of this unit's read-only/no-build mandate.
- recommendation: Defensively install the provider at the top of `main`
  (`rustls::crypto::aws_lc_rs::default_provider().install_default().ok()`),
  or have the dependencies unit run `cargo tree -e features -i rustls` for the
  web binary and record the answer in CODING.md next to the existing rule.

### `gloo-net` dependency is unused
- severity: minor
- category: dependencies
- location: web/Cargo.toml:75
- finding: `gloo-net = { version = "0.7", features = ["websocket"] }` — a
  repo-wide grep of `rust/web` finds zero `gloo_net` references in `src/` or
  `tests/`. The websocket feature in particular is dead weight since the
  client WS moved to leptos-use (`websocket_client.rs`). It's a non-optional
  dependency, so it is compiled into the WASM hydrate bundle for nothing.
  (`gloo-timers` on line 76 IS used — `src/app.rs:189`,
  `src/components/opponent_slot.rs:1` — keep it.)
- recommendation: Delete the `gloo-net` line. (Also note the gloo family is
  effectively in maintenance mode; when convenient, `gloo-timers` could move
  to leptos-use's `use_interval_fn`, which is already a dependency — optional
  cleanup, not required.)

### tokio features rely on transitive unification: `net` and `time` used but not declared
- severity: minor
- category: consistency
- location: web/Cargo.toml:24 vs web/src/main.rs:103,184 (`tokio::net::TcpListener`) and web/src/websocket.rs:128-130,181 (`tokio::time`)
- finding: The `tokio` dependency declares only `["rt-multi-thread",
  "macros", "signal"]`, but this crate directly uses `tokio::net::TcpListener`
  (needs `net`) and `tokio::time::interval`/`timeout` (needs `time`). It
  compiles today only because other dependencies (axum/sqlx/async-nats)
  enable those features and feature unification leaks them in. A dependency
  upgrade that drops a transitive feature would break this crate's build with
  confusing errors.
- recommendation: Add `"net"` and `"time"` to the tokio feature list.

### `futures-util` is non-optional but only used in ssr/test code → compiled into the WASM bundle
- severity: nit
- category: dependencies
- location: web/Cargo.toml:74
- finding: All `futures_util` uses are ssr-side (`src/websocket.rs:27` inside
  `mod ssr`, `src/auth/server.rs` join_all, `src/game/mod.rs` test modules)
  or integration tests, yet the dependency is unconditional, so it joins the
  hydrate bundle's dependency graph. Compile-time/bundle-size cost only.
- recommendation: Make it `optional = true` and add `dep:futures-util` to the
  `ssr` feature (dev/test usage is covered since tests run with `ssr`).

### Dependency currency spot-check (crates.io API, 2026-07-24)
- severity: nit
- category: dependencies
- location: web/Cargo.toml
- finding: Checked newest published versions for the ssr-relevant set.
  Current/at-latest: axum 0.8.9, tower-http 0.7.0, sentry/sentry-tower/
  sentry-tracing 0.48(.5), resend-rs 0.28.0, mrml 6.0.1, mail-parser 0.11(.5),
  reqwest 0.13(.4), pulldown-cmark 0.13.4, petname =3.1.0, codee 0.3.5,
  dotenvy 0.15(.7), leptos-use 0.19.0, time 0.3, getrandom 0.4,
  axum-prometheus 0.10.0, serial_test 3.5.0, tokio-tungstenite 0.30.0.
  Behind: `async-nats` 0.49.1 → 0.50.0 available (line 79); `svix` 1.98 →
  1.99.1 available (line 50). leptos 0.9.0-beta exists but 0.8.x is the
  current stable line — no action. `web-sys 0.3.77`/`js-sys 0.3` lag the
  wasm-bindgen =0.2.121 lockstep releases somewhat (0.3.9x era) but are
  semver-compatible with the pin — cosmetic. sqlx 0.8 / tower-sessions 0.14
  holdbacks are intentional per project note (not flagged).
- recommendation: Bump `async-nats` to 0.50 and `svix` to 1.99.1 at next
  dependency pass; check async-nats 0.50 changelog for JetStream API changes
  before bumping. No unmaintained/deprecated crates spotted in the
  ssr-relevant set beyond the gloo note above.

### WS inbound message/frame limits left at tungstenite defaults
- severity: nit
- category: quality
- location: web/src/websocket.rs:82-87
- finding: `ws.on_upgrade` is used without `.max_message_size()`/
  `.max_frame_size()`, so tungstenite defaults apply (~64 MiB message /
  16 MiB frame). Inbound client messages are drained and ignored — the server
  never needs anything larger than a close/pong — but an anonymous client can
  make each connection buffer up to the max message size. Bounded per
  connection (and the HTTP body limit doesn't apply to upgraded sockets), so
  low risk; Cloudflare edge limits further mitigate.
- recommendation: Set small explicit limits on the upgrade, e.g.
  `ws.max_message_size(4 * 1024).max_frame_size(4 * 1024)`.

### No dead-connection detection beyond send failure (half-open WS can linger)
- severity: nit
- category: quality
- location: web/src/websocket.rs:127-164
- finding: The server sends a Ping every 30s (good — defeats LB idle
  timeouts) but never verifies Pongs and has no read timeout. A silently
  half-open client (laptop sleep, network vanish) keeps its task, gauge
  count, and two NATS subscriptions alive until TCP keepalive/kernel timeout
  (typically hours), because tiny ping writes keep succeeding into the kernel
  buffer. Leak is bounded and self-healing; common practice.
- recommendation: Optional: track last-pong/last-message timestamp and close
  connections idle for >2–3 ping intervals, or rely on tungstenite's
  queued-write limits to eventually error the send.

### import-game CLI: unbounded file read, no input size guard
- severity: nit
- category: quality
- location: web/src/bin/import_game.rs:20
- finding: `std::fs::read_to_string` loads the whole bundle into memory with
  no size cap. Dev-only tool run by hand against trusted local files, so this
  is purely defensive polish.
- recommendation: None required; a size sanity check with a clear error is
  optional.

---

## Clean / verified-good areas (explicitly confirmed)

- **`src/router.rs`** — middleware ordering is correct and matches its
  comments: axum `Router::layer` wraps the *routes registered so far*
  (route-level, so `MatchedPath` IS available inside `make_root_span` and
  `set_sentry_transaction_name` — both verified against axum semantics, no
  cardinality bug); `/healthz` correctly escapes the session layer while
  still getting timeout/body-limit/trace; sentry layer order
  (NewSentry outermost → SentryHttp → transaction-name middleware) is
  correct for per-request hub scoping; `MAX_REQUEST_BODY_BYTES` (256 KiB) and
  30s `TimeoutLayer` are sane and the `/ws`-vs-timeout interaction is
  accurately documented; `ROUTES` LazyLock correctly addresses the
  `IS_SUPPRESSING_RESOURCE_LOAD` race; `set_cache_control` logic (immutable
  hashed `/pkg/`, `no-cache` HTML) matches docs/decisions/ASSET_CACHING.md.
- **`src/main.rs`** — panics/unwraps are startup-only (allowed per project
  rules); tracing→sentry init order is deliberate and correct; sentry
  `send_default_pii: false` and 0.1 trace sampling are set; metrics server
  failure is logged-not-fatal and port 9090 is correctly not exposed in k8s
  (verified against deployment.yaml annotations); graceful-shutdown signal
  handling (SIGINT/SIGTERM) is correct; PrometheusMetricLayer applied once
  per process with the test-motivated comment intact.
- **`src/websocket.rs` (publish side)** — `broadcast_game_update`/
  `broadcast_proposal_update` follow the project rule exactly: serialize
  checked, publish error logged, `.flush().await` after publish, flush errors
  logged not propagated. `WsConnectionGuard` covers all exit paths for the
  gauge. Non-UTF8 NATS payloads are skipped, not fatal. The `ignore`d flaky
  test is already documented with a pointer to the deferred item.
- **`src/state.rs`, `src/config.rs`, `src/error.rs`** — clean; `error::internal`
  correctly logs server-side and returns an opaque client message;
  `FromRef` impls match extractor usage.
- **`src/nats.rs` (shape)** — WorkQueue retention with two disjoint
  filter-subject consumers is valid (no overlapping-subject rejection);
  explicit ack policy is the right choice for bot work;
  `BotTurnEvent`/`BotCommandEvent` schemas are minimal and versionable.
- **`src/websocket_client.rs` (parse path)** — relative `"/ws"` URL is
  correctly normalized to `ws(s)://host/ws` by leptos-use (verified in
  upstream v0.19.0 source); signal parse order (game then proposal) is
  unambiguous because the payload shapes are disjoint; `bump_*_update`
  seq-derivation rationale is sound; `ReconnectLimit::Infinite` appropriate.
- **`src/bin/import_game.rs`** — clean dev CLI: proper usage exit code,
  anyhow error chains with path context, prints resulting URL.
- **`src/auth/blocked_domains.rs` (skim)** — vendored list loaded as a
  lazily-initialized `HashSet<&'static str>`; `is_blocked` lowercases before
  lookup (case-safe); both call sites (`auth/server.rs:294,801`) go through
  `is_blocked`. O(1) lookup, no per-request allocation concerns beyond one
  `to_lowercase`. No issues.
- **`Cargo.toml` feature wiring** — ssr/hydrate feature split is disciplined
  (mrml/pulldown-cmark/petname correctly ssr-only; hydrate correctly minimal);
  `hash-files = true` matches the cache-control strategy; the wasm-bindgen
  pin and sqlx/tower-sessions holdbacks are documented intentional (not
  flagged).

## Coverage statement

All in-scope files were read in full (blocked_domains skimmed per
instructions). `Cargo.toml` reviewed at manifest level only; Cargo.lock and
other crates' manifests were not audited (one lockfile grep solely to confirm
ring/aws-lc-rs presence for the crypto-provider finding). No code was edited
anywhere; no cargo builds or tests were run; the only write is this findings
file.
