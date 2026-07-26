# WP-84: migrate `/ws` to Server-Sent Events

**Findings:** none directly - this supersedes the transport half of ws F59.
**Decisions:** D-44 (commit to SSE, migrate now), D-45 (no `Last-Event-ID`),
D-46 (topology - see `planning/sse-topology-decision.md`, now RESOLVED), D-47
(Cloudflare: rate-limit establishment only, server heartbeat, verify real
config), D-48 (browser leg is HTTP/2 -> **two streams**; never three),
D-49 (future SSE uses: build no topic machinery), D-50 (`/events/public` takes a
repeatable `topic=game:<uuid>` param, N from day one).
**Landing order:** WP-82 (db split) -> WP-47 -> **WP-42's predicate work**
(`is_proposal_visible_to_user` + the TTL cache; its pre-upgrade auth dance is
superseded and must NOT be written) -> **WP-84**. `/events` depends on the
visibility predicates.

**Length justification:** the Tier 2 cap is ~120 lines. This spec is ~300 because
it is a transport migration spanning server routes, auth, shutdown, metrics,
client, Cloudflare/k8s infra, a deletion list and a regression-test list - each
of which would otherwise be its own spec. Reviewed and accepted at this length by
the Lead; the density is verified fact, not padding.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

> **SETTLED - no blockers, no conditionality.** `curl -sI https://brdg.me |
> head -1` returned `HTTP/2 200` (measured by Michael 2026-07-26), so the browser
> leg is HTTP/2 through the Cloudflare edge and the ~6-connections-per-origin cap
> does not bite. **D-48 rules TWO STREAMS.** The single-stream fallback that
> earlier drafts carried is deleted. This spec is one design, not a decision tree.

## 1. Problem

`/ws` (`rust/web/src/websocket.rs`) uses a WebSocket purely as a one-way
server->client pipe: `handle_socket`'s inbound `select!` arm is
`Some(Ok(_)) => {}` under the comment "we don't act on client-sent data here",
and the client (`websocket_client.rs::use_websocket`) never binds the `send`
handle. The 101 upgrade hijacks the connection, which is the *only* reason
WP-42 needs a hand-rolled pre-upgrade auth path, a `TaskTracker`, and a
real-listener test harness. SSE is an ordinary `GET`, so all three problems
dissolve.

## 2. Non-goals

- WP-42 Task B (`sub`/`unsub`). **Do not build it.** There is no client->server
  channel; scope lives in the URL.
- The visibility predicates themselves (WP-47 / WP-42) - consume, do not author.
- `Last-Event-ID` replay, JetStream, an events table.
- Topic machinery beyond `game:` (D-49): no multiplexing layer, no channel
  registry, no subscription protocol, no `tournament:`/`chat:` kinds. The topic
  *parameter* generalises count only, not kind (§3a, §3c).
- A third held stream, now or later (§3a hard cap).
- Any change to the 15 NATS publish sites or to `broadcast_game_update` /
  `broadcast_proposal_update`.

## 3. Required end state - server

### 3a. Routes (`rust/web/src/router.rs::build_router`)

Two routes, registered **in the same block as `/ws`** (`.route(...)` calls
before `.layer(session_layer)`; `/healthz` is deliberately registered *after* so
it bypasses the session layer). Verified live: `/ws` is registered before
`.layer(session_layer)`. Registering `/events` there means tower-sessions wraps
it and a `Session` extractor resolves - **no router or layer reordering.**

- `GET /events` - private, identity-scoped. Opened once per SPA session, **never
  swapped** on navigation.
- `GET /events/public?topic=game:<a>&topic=game:<b>` - unauthenticated. The
  **same key repeated**; no `[]` suffix (that is a PHP/Rails convention, repeated
  keys already carry array-ness and `[]` only adds percent-encoding noise).
  Swapped on navigation.

**Hard cap - never three held streams (D-48/D-49).** Exactly two SSE connections
are held open. Any future SSE use - private chat, notifications, presence - rides
the existing private `/events` stream as a new `event:` name (§3e), not a third
connection. Prod is HTTP/2 so the two are cheap, but **dev is permanently
HTTP/1.1** - both Tilt modes serve plain HTTP, so there is no TLS and therefore
no ALPN, and axum's `http2` feature is enabled nowhere in the workspace, so h2c
is not available either. Two of dev's ~6-per-origin budget is comfortable.
**Any future increase in held-stream count must be re-checked against dev, not
just prod** - that is where the stalls appear that never reproduce in production.

**Why a topic collection today when the UI only ever passes one (D-50):** the
single-game assumption does not stay in the URL. It leaks into the subscription
bookkeeping and the fan-out path, and *that* is the expensive part to undo later.
Parsing into a collection from day one costs a `Vec` and a loop and removes the
trap. Topic *kinds* stay speculative and are rejected (§3c); only topic *count*
is generalised.

`REQUEST_TIMEOUT` needs no change: `TimeoutLayer` races only the inner service
future, which resolves when the `Sse` value is constructed. **Update
`REQUEST_TIMEOUT`'s doc comment** - it currently explains itself in terms of
`WebSocketUpgrade::on_upgrade`.

### 3b. Handler shape (new `rust/web/src/events.rs`; keep `GameBroadcaster` where it is)

Ordinary `async fn` handlers using `axum::response::sse::{Sse, Event, KeepAlive}`
returning `Sse<impl Stream<Item = Result<Event, Infallible>>>`. One shared
private stream-builder used by both routes, parameterised by the per-frame
predicate. It subscribes the **unchanged** wildcards `game.>` and `proposal.>`
on `broadcaster.client`, then `select!`s over: the game subscription, the
proposal subscription, and `broadcaster.shutdown.cancelled()` (which ends the
stream). The ping interval goes away - `KeepAlive` replaces it.
`/events/public` emits no proposal frames (§3c/§3d); whether it also skips the
`proposal.>` subscribe is the implementer's call - dropping them in the predicate
is fine and keeps one builder.

Wrap with `Sse::new(stream).keep_alive(KeepAlive::default())`. **Verified in
axum 0.8.9 `src/response/sse.rs`: `KeepAlive::new()` sets `max_interval` to 15
seconds and emits an empty comment.**

### 3c. Auth

`/events` takes `session: tower_sessions::Session` and `State(pool): State<PgPool>`
as ordinary extractors. Resolve `viewer: Option<Uuid>` via
`auth::session::get_user_from_session` then
`auth::session::validate_session_token`; only `Ok(true)` yields `Some`.
**WP-42 §3a's pre-upgrade dance is deleted, not ported** - there is no 101, so
identity can be resolved anywhere in the handler and the session layer's
response-side save pass runs normally. Do not use `get_current_user` (a
`#[server]` fn whose leptos context does not cover a plain route).
**Never 401 an anonymous connect** - degrade to publicly-visible only.

`/events/public` takes **no session extractor at all**. It takes
`State<PgPool>` and a `Query` of topics (see below). It emits no proposal frames.
Public game ids are already public, so this route has no per-user predicate, no
TTL cache and no identity on the hot path.

#### Topic parsing and validation (D-50)

- Parse into a **collection from day one**: a `Vec<Uuid>` of requested game ids.
- **Accept only `game:<uuid>` topics.** Reject every other kind - no
  `tournament:`, no `chat:`. D-49 is explicit: **no topic machinery, no
  multiplexing layer, no channel registry, no subscription protocol.** The prefix
  exists so a future kind is an additive change, not because a second kind is
  planned.
- **Cap N at 16 (D-52 - Michael's ruling, not a proposal).** This is a guard
  against one connection being made arbitrarily expensive, not a product limit.
  The number is settled: the implementer must use **16**, not a different small
  number. Over the cap -> `400`, never a truncation, so a UI asking for too many
  fails visibly rather than silently dropping topics.
- **Reject, do not silently ignore.** An unknown prefix, a malformed UUID, or a
  topic string with no `:` is a **`400`**, so a client bug surfaces immediately
  instead of appearing as a stream that quietly omits things.
- **Zero topics** (no `topic` param at all, or an empty list) is also a `400`.
  The client must simply not open the stream when there is no game to watch
  (§4), so a zero-topic request is a client bug and should say so. Do not open an
  idle stream that can never emit.

##### How to read repeated keys in axum 0.8.9 - VERIFIED, do not improvise

This is the one genuinely non-obvious implementation detail in the spec.
Verified by reading real crate source this session (extracted from
`~/.cargo/registry/cache/`; there is no `vendor/` dir): `axum-0.8.9/src/extract/
query.rs` -> `serde_urlencoded-0.7.1/src/de.rs` -> `serde_core-1.0.228/src/de/
value.rs` (`MapDeserializer`, `PairDeserializer`) and `de/impls.rs` ->
`form_urlencoded-1.2.2/src/lib.rs` (`Parse::next`). `rust/Cargo.lock` pins
axum **0.8.9** exactly, `axum-core 0.5.6`.

**Use `Query<Vec<(String, String)>>` and filter for key == `"topic"`.** It works
and preserves **every** repeated pair, in query order: `deserialize_seq` calls
`visitor.visit_seq(self.inner)` and `MapDeserializer::next_element_seed` hands
each pair to `PairDeserializer`. serde_urlencoded's own doctest covers this exact
type. **No new dependency, no lockfile change, no feature flag.**

These forms **do not work** - an implementer's first instinct is the struct form,
which fails:

| Form | Result |
| --- | --- |
| `Query<Vec<(String, String)>>` | **works**, keeps all repeats in order |
| `Query<HashMap<String, String>>` | collapses duplicates, **last wins** |
| `Query<HashMap<String, Vec<String>>>` | **400** |
| struct with `topic: Vec<String>` | **400** |

The two failures are the same cause: serde_urlencoded's `Part` value
deserializer forwards `seq` to `deserialize_any`, which only ever visits a
string.

Also settled so it is not re-litigated: `axum-extra` is **absent from
`rust/Cargo.lock` entirely**, not even transitively (latest 0.11.0 per the local
index cache); its repeated-key semantics are UNKNOWN-by-read and it is not
needed. `serde_qs` **is** in the lock at 0.15.0 but only transitively (via
`leptos 0.8.20` / `server_fn 0.8.13`) and latest is 1.1.2, so adding it directly
at latest would put two majors in the tree - **do not add it.** There is no house
pattern to follow: `grep -rn "extract::Query\|RawQuery\|axum_extra"` over `rust/`
returns **zero** hits; `rust/web/src` never parses a query string server-side.
The only query handling anywhere is client-side `use_query_map()` in
`rust/web/src/new_game.rs` and `rust/web/src/players.rs`.

### 3d. Per-frame filtering

`/events`: exactly WP-42 §3b/§3c, unchanged - `is_game_visible_to_viewer` for
game frames, `is_proposal_visible_to_user` for proposal frames, behind the
bounded per-connection TTL cache (~256 entries, 30s, fail closed on `sqlx`
error, `tracing::warn!`, not cached).

`/events/public`: the per-frame test is **set membership over the N requested
ids, not equality against one** - "is this frame's game id in the requested topic
set AND is that game publicly visible" (`db::is_game_publicly_visible`). Hold the
ids in a set built once at connection setup; the hot path is a set lookup plus
the public-visibility check. Still **no per-user predicate, no TTL cache and no
identity on the hot path.** It emits no proposal frames.

### 3e. Event naming and payloads

Reuse `GameUpdateSignal` / `ProposalUpdateSignal` verbatim (skinny UUID JSON).
Replace the client's try-deserialize-then-fall-back guess with **SSE named
events**: `Event::default().event("game").data(payload)` and
`.event("proposal").data(payload)`. `Event::event` is verified present in axum
0.8.9. `/events` emits `game` and `proposal`; `/events/public` emits `game` only.

**Name the `event:` field meaningfully from day one, for every frame - this is a
hard requirement, not a nicety (D-49).** A meaningful event name is exactly what
makes "one private stream carrying multiple message types" work when private chat
or notifications eventually arrive: a new type is a new event name on the
existing stream, with no change to the connection topology and no third held
stream. It costs nothing now. **Never emit an untyped/default message kind.**

### 3f. No `id:` - hard rule (D-45)

**Never call `Event::id()`.** NATS Core has no replay buffer, so emitting `id:`
would make the browser send `Last-Event-ID` on reconnect and promise a gap-fill
the server cannot deliver. Reconnect means "bump the trigger and refetch what is
visible", which the client already does on `visibilitychange` and `online`.

### 3g. Shutdown - what goes, what stays, what to verify

**MUST stay:** `GameBroadcaster::shutdown: CancellationToken`, `begin_shutdown`,
and the stream's `select!` arm on it. Because an SSE stream is an in-flight
request, `axum::serve(..).with_graceful_shutdown(..)` will **not finish until
every stream ends** - if the arm is dropped, shutdown hangs forever. Preserve
`main.rs`'s current ordering: `shutdown_signal().await` then
`broadcaster.begin_shutdown()` *inside* the `with_graceful_shutdown` future.

**Candidates for deletion (only after `/ws` is gone):** `ws_tasks: TaskTracker`,
`track_future` in the handler, `drain_ws_tasks`, the `ws_tasks.close()` line in
`begin_shutdown`, and `main.rs`'s 5s `tokio::time::timeout(.., drain_ws_tasks())`
block. **The evaluation marked this INFERRED and I have not read hyper's
graceful-shutdown implementation - UNKNOWN.** Before deleting, the implementer
MUST prove it with a real-listener test: open a stream, trigger graceful
shutdown, assert the server task completes and the stream ended. If it does not,
keep the tracker.

**Carry over the connection gauge.** Rename `ws_connections` ->
`sse_connections` and `WsConnectionGuard` -> `SseConnectionGuard`, held by the
stream's state so every exit path decrements once.

### 3h. `axum_prometheus` - RESOLVED, no distortion

Read in vendored `axum-prometheus 0.10.0`: `Traffic::on_response` (`src/lib.rs`)
records `data.start.elapsed()` into `axum_http_requests_duration_seconds`, and
`on_response` is called from `lifecycle::future::ResponseFuture::poll` the
moment the **inner service future resolves** - i.e. at response headers, before
any body frame. `axum_http_requests_total` increments there too. **A multi-hour
SSE stream is therefore recorded as a sub-millisecond request. No exclusion is
needed and none should be added.** Also confirmed: `PrometheusMetricLayer::pair()`
constructs `LifeCycleLayer::new(.., None)`, so `BodySizeRecorder` is **off** and
no per-chunk body-size histogram is written.

**One real effect, document it:** `axum_http_requests_pending` is incremented in
`Traffic::prepare` and decremented by `Drop for Pending`, whose `Arc` is cloned
into the streaming `ResponseBody`. So `axum_http_requests_pending{endpoint="/events"}`
counts live streams for their whole lifetime. That is a free equivalent of the
`sse_connections` gauge, but it means "pending requests" on those two endpoints
no longer means "handler still running". Note it wherever the metric is consumed.

## 4. Required end state - client

`rust/web/src/websocket_client.rs`. Keep `WebSocketTrigger`, `ProposalUpdate`,
`bump_game_update`, `bump_proposal_update` and their doc comments **unchanged** -
none of the reactive machinery changes. Replace `use_websocket_with_options`
with `leptos_use::use_event_source_with_options`.

- **`ReconnectLimit::Infinite` must be set explicitly.** Verified in leptos-use
  0.19.0: `UseEventSourceOptions::default()` uses `ReconnectLimit::default()`,
  which is `Limited(3)`. The current WS call site already overrides this; the SSE
  one must too, or streams silently die after three failures.
- Set `.named_events(vec!["game".into(), "proposal".into()])`.
- **API difference to plan for:** `use_event_source` has **no `on_message_raw`**.
  It returns `UseEventSourceReturn { message, ready_state, error, open, close }`
  where `message: Signal<Option<UseEventSourceMessage<T, C>>>` carries
  `event_type` and `data`. Drive the trigger bumps from an `Effect` on `message`,
  branching on `event_type`. **Verify** that two consecutive identical frames
  (same `game_id`) both fire the effect - `set_message.set(..)` on a plain
  leptos signal should notify unconditionally, but if it dedupes, switch to the
  `on_event` option instead. Either way the `(id, seq)` bump must advance per
  frame.
- `open` is still returned, so the existing `visibilitychange` and
  `window_event_listener(online, ..)` handlers keep working as-is.
- Both streams are opened from `app.rs::App`, above `<Router>`, where
  `use_websocket()` is called today. The `/events/public` URL is a
  `Signal<String>` derived from the router location. It builds a **topic list**:
  on the `("games", ParamSegment("id"))` route the list is exactly one entry,
  `topic=game:<id>`; on any other route there is no game to watch, so **do not
  open the stream at all** (a zero-topic request is a 400 by §3c). The URL
  builder must percent-encode and join repeated `topic=` pairs, not assume one -
  the list shape is what keeps a future multi-game view additive. leptos-use
  holds an `Effect::watch` on the URL signal and re-opens on change (verified).
  **Debounce the URL signal** so clicking rapidly through the sidebar does not
  churn streams. **Nothing may still emit `?game=<uuid>`.**
- `rust/web/Cargo.toml`: add `use_event_source` to the leptos-use feature list.
  Verified: its deps (`codee`, `use_event_listener`) are already enabled, so this
  is a one-word edit. Remove `use_websocket` in the deletion commit.

**Rename recommendation:** yes, but **in the deletion commit, not the add
commit.** `websocket_client.rs` -> `events_client.rs`, `WebSocketTrigger` ->
`EventsTrigger`, `use_websocket()` -> `use_events()`. The names are load-bearing
in only a handful of `provide_context`/`expect_context` sites, and leaving
"websocket" in the names of the code that replaced WebSockets is a trap for the
next reader. Deferring it keeps the add commit reviewable and avoids churning
`app.rs`, which is under concurrent edit.

## 5. Rollout: side-by-side, NOT a cutover

**Recommendation: side-by-side, in three commits.** (1) Add `/events` +
`/events/public` alongside `/ws`; both transports live, `/ws` still the one the
client uses. (2) Switch the client, keeping `/ws` serving. (3) Delete `/ws` and
the WS code once no client opens it.

Reasoning: both transports subscribe the same NATS wildcards, so a client on
either sees the same signals and a mixed fleet is consistent. More importantly,
`/pkg/` assets are content-hashed and **edge-cached as `immutable`** (see
`infra/cloudflare.tf`'s `pkg_immutable_assets` cache rule and `router.rs`'s
`set_cache_control`), so a browser holding an old wasm bundle keeps requesting
`/ws` after deploy. A flag-day cutover breaks those clients until they reload;
side-by-side does not. Rollback in step (2) is a one-line client revert.

## 6. Infra (D-47)

- **Verify, do not assume.** Before rollout the implementer must check the live
  Cloudflare config (dashboard or `tofu plan`) and the real proxy idle timeouts.
- **The only idle timeout in this repo** is
  `k8s/base/gateway/gateway.yaml`'s annotation
  `service.beta.kubernetes.io/do-loadbalancer-http-idle-timeout-seconds: "120"`.
  The 15s `KeepAlive` default is comfortably under it. Cloudflare's own edge
  idle behaviour for `text/event-stream` is **UNKNOWN from repo state**.
- **The live rate-limit rule, read from `infra/cloudflare.tf`:** exactly one
  ruleset (`cloudflare_ruleset.rate_limit`, phase `http_ratelimit`) with one rule
  `api_per_ip`, expression `(starts_with(http.request.uri.path, "/api/"))`,
  action `block`, `characteristics = ["cf.colo.id", "ip.src"]`, `period = 10`,
  `requests_per_period = 60`, `mitigation_timeout = 10`. So the prior
  evaluation's claim is **confirmed: `/ws` is exempt today.**
- If a rule is added for SSE it must match **connection establishment only** -
  never stream duration and never bytes streamed (D-47). **`/events/public` must
  stay UNMATCHED by any rate rule:** navigation reopens it on every game-page
  change, and the free tier's fixed 10s period makes that easy to trip. Scope any
  rule to `/events`, which is opened once per SPA session. If a later rule does
  cover `/events/public` anyway, size it for the navigation rate, not the
  page-load rate.
- `cloudflare_zone_setting.websockets = "on"` can be removed once `/ws` is gone
  (its comment says it exists solely for `/ws`). The `fight_mode` comment's
  "may break websockets" caveat becomes stale - leave the resource alone,
  update the comment.

## 7. What gets deleted (final commit)

- `rust/web/src/websocket.rs` - the whole `ssr` module's WS half
  (`ws_handler`, `handle_socket`, `WsConnectionGuard`, the ping interval).
  `GameBroadcaster`, both `broadcast_*` fns and the
  `broadcast_publishes_skinny_signal_to_game_subject_only` test **stay**.
- The `/ws` route in `router.rs`.
- `rust/web/tests/websocket_hygiene.rs`, and its `tokio-tungstenite = "0.30"`
  dev-dependency. Confirmed: `tokio_tungstenite` appears nowhere else in
  `rust/`. Port its intent, not its code (see §8).
- axum's `ws` feature in `rust/web/Cargo.toml`. **Confirmed unused elsewhere:**
  `rust/bot` and `rust/operator` declare plain `axum = "0.8"`, and
  `WebSocketUpgrade`/`extract::ws` appear only in `websocket.rs`.
- leptos-use's `use_websocket` feature.
- `cloudflare_zone_setting.websockets`.
- The `/ws` sentence in `REQUEST_TIMEOUT`'s doc comment.

## 8. Regression test cases

The in-process `tower::ServiceExt::oneshot` harness in
`rust/web/tests/ssr_pages.rs` **can** drive an SSE `GET` (unlike a WS upgrade):
the handler returns a normal `Response` and
`axum::body::Body::into_data_stream()` (verified present in axum-core 0.5.6)
yields frames incrementally with `futures-util`, already a dependency. Copy
`ssr_pages.rs`'s `make_state` / `login_cookie` helpers.

- **Anonymous `GET /events` returns 200 with
  `Content-Type: text/event-stream`** - never 401. Replaces
  `live_websocket_survives_idle_past_request_timeout`'s 101 assertion.
- **A frame for an all-`'public'` game (seeded via the `PgPool`, never a random
  UUID) reaches an anonymous `/events` stream**, as an `event: game` line.
- **A private game's frame does not reach an authenticated non-participant**
  within a short timeout, but a game it *is* a player of does - proving
  liveness, not deadness.
- **Proposal frames:** participant yes; non-participant no; anonymous no; and
  `/events/public` emits none at all.
- **`/events/public?topic=game:<a>`** receives frames for `<a>` but nothing for
  a different game, and nothing for a private one.
- **N>1 topics all deliver:** `?topic=game:<a>&topic=game:<b>` receives frames
  for **both** `<a>` and `<b>`. This is the test that catches a
  `HashMap`-collapsing or struct-based `Query` extractor (§3c) - without it the
  wrong extractor passes every other test.
- **Rejection cases all return `400`,** not a silently-degraded stream:
  a non-`game:` topic (`?topic=tournament:<uuid>`), a malformed topic
  (`?topic=game:not-a-uuid`, and one with no `:` at all), zero topics
  (`/events/public` with no query), and more than the cap.
- **Real-listener test (keep one):** a stream survives past the 30s
  `REQUEST_TIMEOUT` and receives a keepalive comment - keepalive timing and real
  HTTP framing/idle behaviour need an actual `axum::serve` listener, not
  `oneshot`. This is also where §3g's graceful-shutdown proof goes.

## 9. Riders

None.
