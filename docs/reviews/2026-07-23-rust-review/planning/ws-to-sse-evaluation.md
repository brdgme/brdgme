# WebSocket -> SSE: evaluation

> **SUPERSEDED IN ITS RECOMMENDATION - 2026-07-26. Read for evidence, not for
> guidance.** This document recommended **Option C** (one stream,
> `GET /events?game=<uuid>`), with Option B's `/events/public` "held in reserve".
> That recommendation was conditional on the HTTP/2 question, which Michael then
> settled by measurement: `curl -sI https://brdg.me | head -1` -> `HTTP/2 200`.
> **D-48 rules TWO STREAMS** (the Option B shape), and **D-50** replaces
> `?game=<uuid>` with a repeatable `?topic=game:<uuid>` parameter accepting N
> topics. The single-stream shape described below **no longer exists anywhere in
> the design.** The settled design is `specs/WP-84-sse-migration.md`; the
> reasoning is `sse-topology-decision.md`; the rulings are D-44..D-50 in
> `decisions-session3.md`. Everything else here - the crate-source findings, the
> inventory-backed facts, the UNKNOWN markers - stands and is why this file is
> kept.

Companion to `raw/websocket-inventory.md` (treated here as established fact) and
`specs/WP-42-websocket-auth-and-filtering.md`. Everything below is either read from live
source/vendored crate source, or explicitly marked INFERRED / UNKNOWN. No line numbers are
cited: read the named function, and if it does not match, STOP and report.

Crates read for this document (extracted from `~/.cargo/registry/cache` into a scratchpad,
nothing written into the repo or the registry): `axum 0.8.9` (`src/response/sse.rs`,
`Cargo.toml`), `axum-core 0.5.6` (`src/body.rs`), `leptos-use 0.19.0`
(`src/use_event_source.rs`, `Cargo.toml`), `tower-http 0.6.8` (`src/timeout/service.rs`),
`leptos_axum 0.8.10` (`Cargo.toml`). Repo files read: `rust/web/src/router.rs`,
`rust/web/src/websocket_client.rs`, `rust/web/Cargo.toml`, `rust/web/tests/ssr_pages.rs`.

> "We originally used websockets as the previous major version of brdgme had messages sent
> from the client back to the server, however in this new version we only really have
> messages sent from the server back to the client. We had some thoughts about client
> messages for subscribing to specific public games, but instead I want to think about this
> functionality at a higher level and consider switching to SSE and then designing
> subscriptions around that. An example might be having a separate SSE channel that can be
> requested to get public game information (maybe a different URL path). A big motivation
> here being the industry largely moving away from websockets and some modern web frameworks
> only supporting SSE and not WS. There are other benefits too around being closer to the
> HTTP protocol."

---

## 1. Summary and verdict

**Verdict: SSE is a good fit for this codebase and the migration is small - recommended, but
not urgent, and it should be sequenced *after* WP-47 and the visibility-predicate half of
WP-42.** The strongest argument is not the industry-trend one; it is that SSE removes the
exact awkwardness WP-42 Task A exists to work around, and removes the need for WP-42 Task B
(`sub`/`unsub`) entirely. The main risk is the HTTP/1.1 six-connections-per-origin budget if
the design uses multiple concurrent channels - which is precisely the shape Michael floated.

**The crux fact: there is no client -> server application traffic.** The inventory establishes
this from both ends, and I re-read the client side to confirm. Server: `handle_socket`'s only
inbound `select!` arm is `Some(Ok(_)) => {}` under the comment "we don't act on client-sent
data here"; the value is bound to `_`. Client: `websocket_client.rs::use_websocket`
destructures `UseWebSocketReturn { ready_state: _, open, .. }` - the `send` handle falls into
the `..` and is never bound; the heartbeat type params are `(), DummyEncoder` and
`.heartbeat()` is never called, so leptos-use sends no application frames either. The reverse
channel carries only protocol pongs and close frames. WebSocket's defining capability is
therefore entirely unused today.

## 2. What SSE gives here, concretely

- **Auth stops being a special case.** WP-42 §3a exists because `ws.on_upgrade` returns the
  101 and hijacks the connection, so identity must be resolved *before* the upgrade and
  `get_current_user` (a `#[server]` fn) cannot be used on the plain `/ws` route. An SSE
  endpoint is an ordinary `GET`: `Session` and `State<PgPool>` extract normally, the
  tower-sessions response-side save pass runs normally, and returning `401` for a
  would-be-authenticated-but-invalid session is a normal response rather than a pre-upgrade
  dance. `/ws` is already registered before `.layer(session_layer)` in `build_router`, so an
  SSE route in the same position inherits the whole middleware stack unchanged.
- **`EventSource`'s inability to set headers costs nothing.** Auth here is a same-origin
  tower-sessions cookie; a browser `GET` to a same-origin path sends it. leptos-use's
  `with_credentials` (default `false`) only matters cross-origin. Verified in
  `use_event_source.rs`: it constructs `web_sys::EventSourceInit` and calls
  `set_with_credentials`.
- **The payload is a perfect SSE fit.** Two skinny shapes, `{"game_id":"<uuid>"}` and
  `{"proposal_id":"<uuid>"}`, UTF-8 JSON, no binary, no envelope. `axum::response::sse::Event`
  supports `.event(name)`, which would let the two signal kinds be *named events* instead of
  the current try-deserialize-`GameUpdateSignal`-then-fall-back-to-`ProposalUpdateSignal`
  guess. leptos-use exposes this via `UseEventSourceOptions::named_events`.
- **`TimeoutLayer` genuinely does not bite.** Read in `tower-http 0.6.8`
  `src/timeout/service.rs`: `ResponseFuture::poll` races a `Sleep` against the *inner service
  future* only, and the service future for an SSE handler resolves as soon as the `Sse`
  response value is constructed. The streaming body is not timed. So the 30s `REQUEST_TIMEOUT`
  is a non-issue for SSE, for a different reason than it is for WS. No router change needed.
  The router also has no `CompressionLayer` (grepped `rust/web/src` - no hits), so nothing
  in-process buffers or gzips the stream. `set_cache_control` only inserts `Cache-Control`
  when absent, and axum's `Sse::into_response` already sets `no-cache` plus
  `Content-Type: text/event-stream` - no interaction.
- **NATS fan-out is unaffected.** The 15 publish sites, the `game.>` / `proposal.>` wildcard
  subscribes, and the 2-replica topology are all transport-agnostic. Only `handle_socket`'s
  `select!` and its `sender.send(Message::Text(..))` change shape.
- **Cloudflare gets simpler.** `cloudflare_zone_setting.websockets = "on"` in
  `infra/cloudflare.tf` exists solely for `/ws`; the bot-management comment about
  `fight_mode = false` "if it breaks websockets or login" also goes away as a WS-specific
  worry. SSE is an ordinary proxied GET.

## 3. What SSE costs, or cannot do

- **The six-connections-per-origin limit is the real risk, and the protocol picture is
  murky.** Browsers cap concurrent HTTP/1.1 connections per origin at ~6 (general browser
  behaviour, not read from this repo); an open SSE stream occupies one for its whole life,
  shared with asset loads and Leptos server-fn POSTs. Under HTTP/2 it is a stream, not a
  connection, and the cap is ~100. What is readable here:
  - **The origin is HTTP/1.1-only.** `rust/web/Cargo.toml` declares
    `axum = { version = "0.8.9", features = ["ws", "macros"] }` with default features, and
    axum 0.8.9's `[features] default` list is `form, http1, json, matched-path, original-uri,
    query, tokio, tower-log, tracing` - `http2` is absent. Nothing else in `rust/*/Cargo.toml`
    enables it, and `leptos_axum 0.8.10` depends on axum with `default-features = false,
    features = ["matched-path"]`. So hyper-util's auto-server is built without h2 support
    (INFERRED from the feature graph, not from a running binary). This bounds the
    Envoy -> pod leg, not the browser's.
  - **The browser leg is UNKNOWN from this repo**, as the inventory already records. Cloudflare
    almost certainly serves the browser over HTTP/2 or HTTP/3 in prod, which would make the
    ~6 limit moot there - but that is not verifiable from repo config.
  - **Dev is the sharp edge.** `Tiltfile` serves `http://web.brdgme.lvh.me:8080` through a
    Cilium Gateway HTTP listener with no TLS, so ALPN cannot negotiate h2 and the browser will
    use HTTP/1.1 (INFERRED). A design holding 2-3 concurrent SSE channels would eat half the
    dev connection budget and can produce stalls that never reproduce in prod. This is the
    single strongest argument against Michael's multi-channel shape.
- **Proxy buffering.** Envoy does not buffer streamed responses unless a buffer filter is
  configured, and none of `k8s/base/gateway/**` configures one (read: `gateway.yaml`,
  `httproutes.yaml` - no `BackendTrafficPolicy` or filter config at all). Cloudflare's
  behaviour for `text/event-stream` is not configured in `infra/cloudflare.tf` and I cannot
  verify it from this repo - UNKNOWN. Standard mitigations if it bites: keep
  `Cache-Control: no-cache` (axum sets it), never compress the route, and send an initial
  event or comment immediately on connect so the first byte flushes.
- **No binary frames.** Irrelevant here. There is no binary payload anywhere on the socket and
  none is contemplated; base64 escape hatches exist if that ever changes.
- **Keepalive.** `axum::response::sse::KeepAlive` defaults to a 15s interval emitting an empty
  comment, and is applied via `Sse::keep_alive(..)` (requires axum's `tokio` feature, which is
  on by default and enabled here). That is stricter than today's 30s WS ping and comfortably
  under the DO LB's `do-loadbalancer-http-idle-timeout-seconds: "120"`. Note the *reason*
  differs: the WS ping keeps a hijacked TCP connection alive; the SSE comment keeps an
  in-flight HTTP response alive. Both satisfy the LB.
- **Reconnect and `Last-Event-ID`: do not offer replay.** The browser reconnects `EventSource`
  automatically and re-sends `Last-Event-ID` *only if the server emitted `id:` fields*.
  `Event::id()` and `Event::retry()` both exist in axum 0.8.9. But the fan-out is **NATS Core,
  not JetStream** - there is no replay buffer, so a `Last-Event-ID` gap-fill would need a new
  backing store (an events table, or moving the signal onto JetStream). **Recommendation: do
  not emit `id:` at all.** The payloads are cache-invalidation pings, so the correct
  post-reconnect behaviour is "bump the trigger once and refetch", which the client already
  does on `visibilitychange` and `online`. Emitting `id:` without replay would be a lie.
- **Reconnect behaviour is split between two layers** (read in leptos-use 0.19.0): the browser
  auto-reconnects, and leptos-use only steps in when `es.ready_state() == 2` (permanently
  closed), retrying after `reconnect_interval` (default 3000ms) up to `reconnect_limit`
  (**default `Limited(3)`** - the WS call site currently overrides this to `Infinite`, and an
  SSE port must do the same or connections silently die after three failures).
- **Server-side cost.** One held-open HTTP request per stream per replica, versus one hijacked
  socket today - roughly the same memory and one fd either way. Two differences:
  - **Shutdown gets simpler, not harder.** WS needs the `TaskTracker` precisely *because* the
    101 detaches the socket from axum's request tracking. An SSE stream is an in-flight
    request, so `axum::serve(..).with_graceful_shutdown(..)` tracks it natively - but that
    also means the server will not finish shutting down until every stream ends, so the
    streams must still `select!` on `broadcaster.shutdown` and terminate. The existing
    `CancellationToken` covers that; `TaskTracker`, `drain_ws_tasks` and the 5s drain timeout
    could likely be retired (INFERRED - would need verifying against hyper's graceful
    shutdown behaviour before deleting anything).
  - **Metrics distortion.** `axum_prometheus::PrometheusMetricLayer` wraps the router in
    `main.rs`. A multi-hour SSE request may be recorded as a multi-hour request latency in the
    HTTP histograms, which WS requests are not (their 101 returns instantly). I did not read
    axum-prometheus's source - **UNKNOWN**, but it must be checked before rollout, and the
    `ws_connections` gauge / `WsConnectionGuard` pattern should be carried over.

## 4. On "modern frameworks only support SSE, not WS"

**Honestly: that claim does not bite on this stack today.** axum 0.8.9 ships first-class
support for *both* - `src/response/sse.rs` (`Sse`, `Event`, `KeepAlive`) and
`src/extract/ws.rs` are both present, and the `ws` feature is already enabled here. On the
client, leptos-use 0.19.0 ships both `use_websocket` and `use_event_source`, and
`web_sys::WebSocket` and `web_sys::EventSource` are both available as raw fallbacks. Neither
half of the stack is pushing this codebase off WebSockets.

Where it *could* matter, fairly stated:

- Edge/serverless runtimes (Cloudflare Workers, Vercel/Netlify functions, Deno Deploy, Lambda
  response streaming) commonly support streamed HTTP responses but not WebSocket upgrades, or
  support them only through a separate stateful product. If brdgme ever leaves self-hosted k8s
  for one of those, an SSE-shaped design ports and a WS-shaped one does not.
- HTTP-shaped traffic is easier to observe and defend: it appears in access logs with a status
  and a route, Cloudflare rate-limiting rules (currently `starts_with(..., "/api/")` only, so
  `/ws` is unprotected at the edge) can be extended to cover it by path, and no
  `websockets = "on"` zone setting is required.
- It removes one Cloudflare feature dependency and one documented failure mode
  (`fight_mode` breaking websockets).

That is a real but modest set of benefits. **The stronger case for SSE here is the one Michael
made first**: the reverse channel is unused, so WebSocket is paying protocol complexity for a
capability the app does not use. The "frameworks" argument should be treated as a tiebreaker,
not the justification.

## 5. Subscription design options

All three assume the same server core: an `async_stream`-style `Stream<Item = Result<Event,
Infallible>>` that `select!`s over `game.>`, `proposal.>`, and `broadcaster.shutdown`, applies
a visibility predicate per frame, and is wrapped in `Sse::new(..).keep_alive(KeepAlive::new())`.

**Option A - single identity-scoped stream (`GET /events`).**
Straight SSE port of WP-42 Task A and nothing more.
- URL: `/events`. Auth: `Session` extractor; anonymous allowed, degraded to publicly-visible
  games only. Reconnect: browser-native, then leptos-use with `ReconnectLimit::Infinite`.
- Connections per client: **1**. No HTTP/1.1 pressure.
- Fan-out: unchanged wildcard subscribes, per-frame predicate + the WP-42 TTL cache.
- Gap: a non-participant viewing a public game page still gets no targeted signal beyond what
  the public predicate allows - the same gap WP-42 Task B was invented to close.

**Option B - Michael's separate public channel (`GET /events` + `GET /events/public`).**
- URLs: `/events` (identity-scoped, as A) plus `/events/public` for public game information.
  Auth: `/events/public` needs none; it could even be served to logged-out clients and cached
  differently. Reconnect: independent per channel.
- Connections per client: **2**, or 3 if a third channel appears. Under HTTP/1.1 this consumes
  a third of the budget; in dev (plain HTTP, likely h1) that is a genuine hazard.
- Fan-out: `/events/public` subscribes `game.>` and filters on "publicly visible" only - no
  session, no per-user cache, so it is cheap and could even be shared/broadcast internally.
- Real advantage: clean separation of concerns and independent caching/auth semantics. Real
  cost: connection count, two client lifecycles to reason about, and duplicate delivery of any
  game that is both public and one the viewer participates in (needs client-side dedup - the
  existing `(id, seq)` bump already tolerates redundant refetches, so this is mild).

**Option C - single stream, subscription encoded in the URL (`GET /events?game=<uuid>`).**
The subscription *is* the request. Navigating to a game page changes the URL signal, which
tears down and reopens the stream with the new scope; the server unions "visible to this
session" with "this specific publicly-visible game".
- URL: `/events`, optionally `?game=<uuid>` (or `?proposal=<uuid>`). Auth: as A.
- Reconnect: browser-native, and the query string is part of the URL so scope survives
  reconnects for free - no re-subscription protocol, ever.
- Connections per client: **1**.
- Fan-out: identical to A, with the predicate widened by the query param (`sub` never bypasses
  the predicate - same rule as WP-42 §3d).
- **Verified support:** leptos-use 0.19.0's `use_event_source(url: impl Into<Signal<String>>)`
  holds an `Effect::watch` on the URL signal that calls `close()`, re-inits and `open()`s
  whenever the URL changes. So a reactive scope is a first-class feature, not a hack.
- Cost: rapid navigation causes stream churn (mitigate by debouncing the URL signal), and each
  reopen is a fresh session lookup and two NATS subscribes.

**Recommendation: Option C, with Option B's `/events/public` held in reserve.** C gets the
subscription semantics Michael wants at a higher level (scope is declarative, in the URL,
reconnect-safe) while holding exactly one connection, which sidesteps the HTTP/1.1 question
entirely - including in dev, where it is most likely to bite. It also makes WP-42 Task B
unnecessary: there is no `sub`/`unsub` protocol to design, because there is no client->server
channel at all, which is the property that made SSE attractive in the first place. Option B
becomes the right answer only if the public feed grows semantics genuinely different from the
private one (different cache-control, different payload shape, unauthenticated CDN caching) -
at which point it is a deliberate second channel rather than a subscription mechanism.

## 6. Migration path and interaction with WP-42

WP-42 splits cleanly, and the two halves have opposite fates:

- **Task A's transport half is superseded.** The `ws_handler` pre-upgrade auth dance (resolve
  `get_user_from_session` + `validate_session_token` *before* `ws.on_upgrade` because the 101
  hijacks the connection, avoid `get_current_user` because leptos context does not cover
  `/ws`) is machinery whose entire reason for existing is the upgrade. On SSE it collapses to
  ordinary extractors. Doing it on WS first and then migrating means writing it twice.
- **Task A's predicate half is NOT wasted.** WP-47's `is_game_visible_to_viewer`, the new
  `is_proposal_visible_to_user` (`EXISTS` over `game_proposal_players`), the bounded per-socket
  TTL cache design, the fail-closed-on-`sqlx`-error rule and the accepted <=30s staleness are
  all transport-independent. They transfer verbatim to an SSE stream's per-frame filter.
- **Task B (`sub`/`unsub`) is eliminated by Option C**, and would be redesigned - not
  ported - under Option B. Do not build it.

**Recommended ordering:**

1. **WP-47 first** (unchanged - WP-42 already declares this dependency; the predicate
   dispatcher must not be forked).
2. **Land WP-42's predicate work** (`db.rs` additions + the TTL cache), wired into the
   *existing* WS handler. This is the security fix and should not wait on a transport
   decision.
3. **Decide the transport.** If SSE: implement `/events` as a second route alongside `/ws`,
   reusing the predicate module verbatim.
4. **Drop Task B.** Deliver the public-game scoping as Option C's query param on the SSE
   route.

Alternatively, if Michael wants to avoid writing the pre-upgrade auth at all, swap 2 and 3:
build `/events` first with auth + filtering, and retire `/ws` once the client is switched.
That saves the throwaway upgrade-auth code but delays the security fix behind a transport
migration - **my recommendation is not to couple them.**

**Side-by-side is possible and is the right rollout.** `/events` can be added as a new route
while `/ws` keeps working; both subscribe to the same NATS wildcards, so a client on either
transport sees the same signals. The client cutover is a one-line swap in
`websocket_client.rs` (`use_websocket_with_options` -> `use_event_source_with_options`, keeping
the same `on_event`/message handling that bumps `WebSocketTrigger`, `bump_game_update` and
`bump_proposal_update` - none of that reactive machinery changes). `/ws` and its Cloudflare
zone setting are deleted in a later commit once no client opens it.

**Client dependency change is one line, verified:** leptos-use 0.19.0's
`use_event_source = ["dep:codee", "use_event_listener", "web-sys/EventSource",
"web-sys/EventSourceInit"]`. `codee` is already a dependency and `use_event_listener` is
already enabled in `rust/web/Cargo.toml`, so the only edit is adding `use_event_source` to the
feature list (and eventually removing `use_websocket`).

**Test story.** `rust/web/tests/websocket_hygiene.rs` needs an SSE analogue, and it can be
*cheaper* than the current one. The `oneshot` harness in `tests/ssr_pages.rs` cannot drive a
WS upgrade (documented in `websocket_hygiene.rs`'s module doc) because `tower::ServiceExt::
oneshot` calls the service directly with no hyper connection to hijack. An SSE `GET` has no
such problem: the handler returns a normal `Response` whose body streams, and
`axum::body::Body::into_data_stream()` (verified present in `axum-core 0.5.6`
`src/body.rs`) yields frames incrementally using `futures-util`, which is already a
dependency - no new dev-dep, no real listener, no `tokio-tungstenite`. That reasoning holds
for assertions about *events arriving*; assertions about **keepalive timing** would still want
`tokio::time` pausing or a real listener, and anything asserting real HTTP framing/idle
behaviour end-to-end should keep a real-listener test.

**Effort/risk sizing (rough, my estimate):** server route + stream + keepalive + shutdown arm:
small, comparable to `handle_socket` today. Client swap: small. Predicate/filter work: already
sized under WP-42 and unchanged. Test port: small-to-medium. Infra: no k8s change required;
one Cloudflare setting removed at the end. **Main risk is not code - it is the unverifiable
HTTP protocol version on the browser leg**, which Option C's single connection largely
neutralises.

## 7. Decisions needed from Michael

1. **Transport: commit to SSE, or keep WS?** Everything else follows. My recommendation is
   SSE, on the grounds that the reverse channel is unused and it deletes WP-42 Task B.
2. **Subscription shape: Option C (query param on one stream) or Option B (separate
   `/events/public` path)?** I recommend C; B is defensible if the public feed is meant to
   have genuinely different auth/caching semantics later.
3. **Ordering: land WP-42's filtering on WS now and migrate after, or migrate first and land
   filtering only on SSE?** I recommend the former (do not block a security fix on a transport
   change) - but it does mean the pre-upgrade auth code in WP-42 §3a is written and then
   thrown away. This is the only genuine waste, and it is small.
4. **Is `Last-Event-ID` replay wanted?** Recommendation: no, and therefore do not emit `id:`.
   Offering it would require moving the signal off NATS Core onto JetStream or a new table.
   Confirm you are happy with "reconnect = refetch everything visible".
5. **Should `/events` be rate-limited at the Cloudflare edge?** `/ws` is currently exempt
   (the rule is `/api/` only). SSE makes path-based limiting trivial; whether it is wanted is
   a call I cannot make.
