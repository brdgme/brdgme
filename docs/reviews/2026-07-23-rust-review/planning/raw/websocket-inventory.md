# WebSocket implementation inventory (LIVE code, 2026-07-26)

Ground truth for a later SSE-migration evaluation. **Facts only, no recommendations.**

Everything below was read directly unless marked INFERRED or UNKNOWN. Line numbers are
omitted deliberately; functions/types are named instead. Implementers: read the named
function; if it does not match, STOP and report.

Files read in full or in the relevant part:
`rust/web/src/websocket.rs`, `rust/web/src/websocket_client.rs`, `rust/web/src/router.rs`,
`rust/web/src/state.rs`, `rust/web/src/main.rs` (server bootstrap + shutdown),
`rust/web/src/app.rs` (context provision, HomePage, GamePage, `track_game_seq`),
`rust/web/src/proposals.rs` (InvitePage, `track_proposal_seq`, publish sites),
`rust/web/src/game/mod.rs` (`broadcast_and_trigger`), `rust/web/tests/websocket_hygiene.rs`,
`rust/web/Cargo.toml`, `rust/Cargo.lock` (versions), `k8s/base/**`, `k8s/prod/**`,
`k8s/dev/**`, `Tiltfile`, `infra/cloudflare.tf`, `docs/ARCHITECTURE.md`, and the vendored
`leptos-use 0.19.0` `src/use_websocket.rs` (extracted from the crates.io cache to a
scratchpad, not into the repo).

---

## 1. Endpoint & framework

- **Route:** `.route("/ws", axum::routing::get(crate::websocket::ws_handler))` in
  `rust/web/src/router.rs::build_router`. Single WS route; no other WS endpoints exist.
- **Framework:** `axum` 0.8.9 (Cargo.lock), with the `ws` and `macros` features enabled in
  `rust/web/Cargo.toml`, under the `ssr` feature. `hyper` 1.10.1. No separate WS crate on
  the server side; `axum::extract::ws` is used directly.
- **Handler signature** (`rust/web/src/websocket.rs`, `mod ssr`):

  ```rust
  pub async fn ws_handler(
      ws: WebSocketUpgrade,
      State(broadcaster): State<GameBroadcaster>,
  ) -> impl IntoResponse
  ```

  `GameBroadcaster` is pulled from `AppState` via `impl FromRef<AppState> for GameBroadcaster`
  in `rust/web/src/state.rs`. The body is
  `ws.on_upgrade(move |socket| tracker.clone().track_future(handle_socket(socket, broadcaster)))`
  — the socket task is registered with a `tokio_util::task::TaskTracker` for shutdown draining.

- **Auth at handshake: NONE.** The handler extracts only `WebSocketUpgrade` and `State`.
  There is no session extractor, no cookie inspection, no user lookup, no origin check, and
  no subprotocol negotiation anywhere in `ws_handler` or `handle_socket`. Any unauthenticated
  client that can reach `/ws` receives the full signal stream (see §5 — the payloads are bare
  UUIDs only, no game content).
- **Middleware position.** `/ws` is registered *before* the `.layer(...)` calls in
  `build_router`, so in axum's ordering every one of these applies to it:
  `create_session_layer` (tower-sessions, Postgres-backed), `set_cache_control`,
  `RequestBodyLimitLayer(256 KiB)`, `TimeoutLayer(30s, 408)`, `TraceLayer`,
  `set_sentry_transaction_name`, `SentryHttpLayer`, `NewSentryLayer`. `main.rs` additionally
  wraps the built router in `axum_prometheus::PrometheusMetricLayer`.
  (`/healthz` is registered *after* `session_layer`, so it alone bypasses sessions.)
  The session layer therefore runs on the handshake request — it loads/creates a session row —
  but the handler ignores the result.
- **Timeout interaction.** `router.rs`'s comment on `REQUEST_TIMEOUT` states that
  `WebSocketUpgrade::on_upgrade` returns the 101 immediately and detaches the socket, so the
  30s `TimeoutLayer` does not bound socket lifetime. `tests/websocket_hygiene.rs` asserts this
  empirically (see §7).
- **Metrics.** `WsConnectionGuard` increments the `ws_connections` gauge on construction and
  decrements on `Drop`, giving one decrement per `handle_socket` exit path.

## 2. Server -> client messages

There are exactly **three** things the server ever sends on the socket, all from the
`tokio::select!` loop in `handle_socket`:

1. **`Message::Text`** carrying the raw NATS payload, forwarded verbatim as a UTF-8 string.
   Two payload shapes exist, both defined at the top of `rust/web/src/websocket.rs` outside
   the `ssr` gate (so the WASM build sees them too):

   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct GameUpdateSignal { pub game_id: Uuid }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ProposalUpdateSignal { pub proposal_id: Uuid }
   ```

   Serde default representation, `serde_json::to_vec`. Wire forms are exactly
   `{"game_id":"<uuid>"}` and `{"proposal_id":"<uuid>"}`. No envelope, no type tag, no
   sequence number, no timestamp. The client disambiguates by attempting to deserialize
   `GameUpdateSignal` first and falling back to `ProposalUpdateSignal`.

   No game state, log text, or user data is ever sent over the socket. The signal is a
   cache-invalidation ping; the client then refetches via Leptos server functions.

2. **`Message::Ping(Vec::new().into())`** — an empty-payload protocol ping every 30s from
   `tokio::time::interval(Duration::from_secs(30))` with
   `MissedTickBehavior::Delay`; the immediate first tick is consumed before the loop.

3. **`Message::Close(None)`** — sent once when `broadcaster.shutdown` (a
   `CancellationToken`) is cancelled, then the loop breaks.

There are no binary frames, no server-sent pongs in application code (axum/tungstenite
auto-replies to client pings at the protocol layer — INFERRED, not read in tungstenite's
source).

## 3. Client -> server messages — THE CRUX

**There is no client -> server application traffic. Michael's belief is correct.**

Evidence, server side. The only inbound arm in `handle_socket`'s `select!` is:

```rust
// Drain inbound messages so pongs and close frames are processed; we don't
// act on client-sent data here.
incoming = receiver.next() => {
    match incoming {
        Some(Ok(_)) => {}
        _ => break,
    }
}
```

The received value is bound to `_` and discarded. There is no `Message::Text(...)` arm, no
deserialization of inbound data, no subscribe/unsubscribe handling, no auth frame handling,
no app-level ping/pong handling. `receiver` is used nowhere else in the file. The only
behavioural consequence of inbound traffic is that `None`/`Err` (stream end or error) breaks
the loop and tears the connection down.

Evidence, client side. `rust/web/src/websocket_client.rs::use_websocket` destructures:

```rust
let UseWebSocketReturn {
    ready_state: _,
    open,
    ..
} = use_websocket_with_options::<String, String, FromToStringCodec, (), DummyEncoder>(
    "/ws",
    UseWebSocketOptions::default()
        .reconnect_limit(ReconnectLimit::Infinite)
        .on_message_raw(move |text: &str| { ... }),
);
```

The `send` closure returned by `use_websocket_with_options` is dropped by the `..` and is
never bound or called. `grep` for `use_websocket`/`WebSocketTrigger` across `rust/web/src`
turns up no other socket construction site. The heartbeat type parameters are `(), DummyEncoder`
and `.heartbeat::<..>(..)` is never called, so leptos-use's optional application-level
heartbeat (which *would* send frames) is disabled — confirmed against leptos-use 0.19.0's
`use_websocket.rs`, where `start_heartbeat` is a no-op when `options.heartbeat` is `None`.

Protocol-level frames:

- **Pings:** server -> client only, every 30s, empty payload (§2). The browser's WebSocket
  implementation answers with a Pong automatically; that Pong is invisible to JS and lands in
  the discarded `Some(Ok(_))` arm above. (Browser auto-pong is INFERRED from the WebSocket
  protocol/browser API, not read in code.) Nothing in the client sends a ping.
- **Close frames:** both directions are possible.
  - Server -> client: `Message::Close(None)` on graceful shutdown.
  - Client -> server: leptos-use calls `web_socket.close()` in two places — the `on_cleanup`
    handler when the component unmounts, and at the top of `connect()` before opening a
    replacement socket. Both emit a normal close handshake from the browser.

So: the socket is a strictly one-way application channel (server -> client JSON signals),
with the reverse direction carrying only protocol-layer pongs and close frames.

## 4. Client consumption (Leptos)

- **Crate:** `leptos-use` 0.19.0, `default-features = false`, features
  `use_websocket`, `use_event_listener`, `use_document`; plus `codee` 0.3.5. `gloo-net`
  (websocket feature) and `gloo-timers` are also dependencies of `rust/web` but the WS client
  path does not reference them — UNKNOWN whether anything else uses gloo-net's websocket
  module (not searched exhaustively).
- **SSR/hydrate gating:** `use_websocket()` in `rust/web/src/websocket_client.rs` is
  `#[cfg(feature = "hydrate")]`, with a `#[cfg(not(feature = "hydrate"))] pub fn use_websocket() {}`
  no-op fallback. So the socket exists only in the WASM/browser build; SSR renders never open
  one. `App()` in `app.rs` calls `crate::websocket_client::use_websocket()` unconditionally at
  component top level, relying on that cfg split.
- **URL:** the literal `"/ws"`. leptos-use's `normalize_url` turns a leading-`/` path into
  `{ws|wss}://{location.host}/ws` using `detect_protocol()` (read in leptos-use source).
- **Connection lifecycle:** `immediate` defaults to `true`, so the socket opens via an
  `Effect` on mount. `reconnect_limit(ReconnectLimit::Infinite)`. **Backoff: none** —
  leptos-use's `reconnect_interval` defaults to `3000` ms and this call site does not override
  it, so reconnect is a flat 3s retry, forever. Reconnect is triggered from both the `onerror`
  and `onclose` closures, guarded by `manually_closed_ref`.
- **Event handlers registered by `use_websocket()`:**
  - `use_event_listener(use_document(), visibilitychange, ...)`: when
    `document.visibility_state() == Visible`, calls `open()` (which resets the reconnect
    counter and reconnects) and bumps `trigger.set_last_update`.
  - `window_event_listener(leptos::ev::online, ...)`: calls `open()` and bumps
    `trigger.set_last_update`.
  Note `open()` is called unconditionally on those events, not gated on `ready_state`;
  leptos-use's `connect()` closes any existing socket first.
- **Frame handling:** the single `on_message_raw` closure tries
  `serde_json::from_str::<GameUpdateSignal>` then `::<ProposalUpdateSignal>`; on either
  success it bumps `trigger.set_last_update` (`|n| *n += 1`) and calls
  `bump_game_update` / `bump_proposal_update`. Unparseable text is silently ignored.
- **Reactive state driven by the socket** (three contexts, all provided in `App()` in
  `app.rs`):
  - `WebSocketTrigger { last_update: ReadSignal<u64>, set_last_update: WriteSignal<u64> }` —
    a global monotonic counter.
  - `RwSignal<Option<(Uuid, u64)>>` (unnamed context) — the *game* update pair
    `(game_id, seq)`.
  - `ProposalUpdate(pub RwSignal<Option<(Uuid, u64)>>)` — the *proposal* update pair.
  `bump_game_update` / `bump_proposal_update` derive the next seq from the current context
  value (`prev + 1`) rather than a separate counter; the doc comment explains this is so the
  `PartialEq`-deduping memos cannot silently drop a refetch.
- **Consumers of `trigger.last_update`** (grep for `last_update.get()`):
  - `app.rs::App` — the `active_games` `LocalResource` (`get_sidebar_games`), provided as
    context and used by the sidebar/layout header.
  - `app.rs::HomePage` — the `public_index` `LocalResource` (`get_public_index`).
  Writers additionally include post-action effects in `components/game.rs` (undo, concede, and
  further actions in the same block) and `websocket_client.rs`'s visibility/online handlers.
- **Consumers of the per-entity `(id, seq)` pairs:**
  - `app.rs` GamePage: `seq_for_this_game = Memo::new(|prev| track_game_seq(prev, game_id(), game_update.get()))`.
    `track_game_seq` keeps the previous seq when the update is for a *different* game, so other
    games' signals do not refetch this page. It keys `game_data`
    (`Resource::new_blocking(move || seq_for_this_game.get(), ...)` -> `get_game_details`) and
    the `logs` `LocalResource` (`get_game_logs`). Unit tests
    `track_game_seq_retains_seq_on_other_game_updates` and
    `track_game_seq_resets_when_viewed_game_changes` cover it.
  - `proposals.rs` InvitePage: `seq_for_this_proposal` via `track_proposal_seq`, keying the
    `proposal_data` `LocalResource` (`get_proposal`).
- **Deliberate redundancy:** `bump_game_update`'s doc comment states that post-action success
  effects bump locally *as well as* the server signal arriving, causing one redundant refetch
  when the socket is up; gating on `ready_state` was rejected because it re-opens a
  half-open-socket window.

## 5. Fan-out

- **Broadcaster type** (`rust/web/src/websocket.rs`):

  ```rust
  #[derive(Clone)]
  pub struct GameBroadcaster {
      client: async_nats::Client,
      shutdown: CancellationToken,
      ws_tasks: TaskTracker,
  }
  ```

  Stored on `AppState` (`rust/web/src/state.rs`) and also `provide_context`-ed into every
  Leptos server-fn render in `build_router`'s `leptos_routes_with_context` closure. NATS crate
  is `async-nats` 0.49.1; this path uses **NATS Core pub/sub**, not JetStream. (JetStream lives
  on `AppState.jetstream` and carries the separate `bot.>` work queue defined in
  `rust/web/src/nats.rs` — `STREAM_NAME`/`bot.turn`/`bot.command`. That is a different
  mechanism from the WS fan-out.)

- **Publish API:** two methods, both fire-and-forget with `tracing::error!` on failure and an
  explicit `client.flush().await` after each publish:
  - `broadcast_game_update(game_id)` -> subject `game.{game_id}`, payload `{"game_id":"..."}`.
  - `broadcast_proposal_update(proposal_id)` -> subject `proposal.{proposal_id}`, payload
    `{"proposal_id":"..."}`.

- **Per-connection subscriptions:** every `handle_socket` invocation opens **two wildcard
  subscriptions** on the shared NATS client:

  ```rust
  let mut game_sub = broadcaster.client.subscribe("game.>").await ...;
  let mut proposal_sub = broadcaster.client.subscribe("proposal.>").await ...;
  ```

  A subscribe failure logs and returns early (dropping the connection).

- **Per-connection filtering: NONE.** Every connected client receives every game and every
  proposal signal in the whole system. There is no user id, no game membership check, no
  subject narrowing. Filtering is entirely client-side: `track_game_seq` /
  `track_proposal_seq` discard ids that do not match the currently-viewed entity, and the
  server-fn refetch is where authorization actually happens.

- **Publish call sites — 15 direct calls to the two broadcast methods** (excluding
  `websocket.rs` itself and test files):

  | File | Enclosing fn | Method |
  |---|---|---|
  | `rust/web/src/game/mod.rs` | `broadcast_and_trigger` | game |
  | `rust/web/src/game/server_fns.rs` | `restart_game_with_roster` | proposal |
  | `rust/web/src/game/server_fns.rs` | `restart_game_with_roster` | game |
  | `rust/web/src/game/server_fns.rs` | `force_delete_game` | game |
  | `rust/web/src/proposals.rs` | `create_proposal` | proposal |
  | `rust/web/src/proposals.rs` | `respond_proposal` | proposal |
  | `rust/web/src/proposals.rs` | `start_proposal` | proposal |
  | `rust/web/src/proposals.rs` | `add_proposal_player` | proposal |
  | `rust/web/src/proposals.rs` | `cancel_proposal` | proposal |
  | `rust/web/src/proposals.rs` | `remove_proposal_slot` | proposal |
  | `rust/web/src/proposals.rs` | `transfer_proposal_ownership` | proposal |
  | `rust/web/src/email/commands.rs` | `run_restart` | game |
  | `rust/web/src/email/commands.rs` | `run_restart` | proposal |
  | `rust/web/src/email/inbound.rs` | `handle_invite_reply` | proposal |
  | `rust/web/src/email/sweep.rs` | `sweep_invite_auto_decline_once` | proposal |

  Game updates are overwhelmingly funnelled through the shared epilogue
  `crate::game::broadcast_and_trigger(pool, broadcaster, jetstream, game_id)`
  ("Broadcasts the skinny game-update signal and triggers any bots whose turn it now is"),
  which has **13 call sites**: `game/mod.rs` (1, inside `execute_command`'s flow),
  `game/server_fns.rs` (4), `email/commands.rs` (5), `proposals.rs` (2), `email/inbound.rs` (1).
  So the effective game-update publish surface is ~14 paths, the proposal surface 11.

- **Shutdown / drain:**
  - `GameBroadcaster::begin_shutdown()` cancels the `CancellationToken` and calls
    `ws_tasks.close()`.
  - `GameBroadcaster::drain_ws_tasks()` awaits `ws_tasks.wait()`.
  - `main.rs` wires `axum::serve(...).with_graceful_shutdown(async { shutdown_signal().await; broadcaster.begin_shutdown(); })`,
    then after serve returns awaits `drain_ws_tasks()` under a 5s
    `tokio::time::timeout`, logging `"websocket tasks did not drain within 5s of shutdown"`
    on expiry.

## 6. Infra touchpoints

- **Prod path (what is actually deployed):** `k8s/prod/app/kustomization.yaml` includes
  `../../base/brdgme` and `../../base/gateway`. Traffic is
  **Cloudflare -> DigitalOcean LB -> Gateway API (Cilium/Envoy) -> Service `web` :3000**.
  - `k8s/base/gateway/gateway.yaml`: `gatewayClassName: cilium`; listeners `web-http` (:80,
    HTTP, `brdg.me`) and `web` (:443, HTTPS, `brdg.me`) with cert-manager-issued
    `brdg-me-tls`. **TLS terminates at the Gateway** (and separately at Cloudflare's edge —
    `ssl = "strict"` in `infra/cloudflare.tf` means CF re-originates TLS to the Gateway and
    validates its Let's Encrypt cert).
  - WS-relevant annotation on the Gateway's generated LB Service:
    `service.beta.kubernetes.io/do-loadbalancer-http-idle-timeout-seconds: "120"`, with the
    in-repo comment: *"Default is 60s. The monolith already pings every 30s
    (rust/web/src/websocket.rs) to keep long-lived WS connections alive across LB idle
    timeouts - this just adds margin."*
  - `k8s/base/gateway/httproutes.yaml`: `web-http-redirect` (301 to https) and `web`
    (`backendRefs: name: web, port: 3000`). No path-level rule for `/ws`; it rides the
    catch-all rule.
  - `k8s/base/web/deployment.yaml`: `replicas: 2`, containerPort 3000 (+9090 metrics, no
    Service/route). Readiness and liveness probes both `GET /healthz` on 3000, period 10s.
    Two replicas means WS connections are spread across pods, which is why the NATS fan-out
    exists.
  - `k8s/base/web/service.yaml`: ClusterIP, port 3000 -> targetPort 3000.
- **Orphaned nginx config:** `k8s/base/ingress/ingress.yaml` carries
  `nginx.ingress.kubernetes.io/proxy-read-timeout: "604800"` and
  `proxy-send-timeout: "604800"` with `ingressClassName: nginx`, and
  `k8s/base/ingress-nginx/` pulls in the ingress-nginx controller v1.0.0 manifest. **Neither
  directory is referenced by any kustomization** in `k8s/prod/`, `k8s/dev/`,
  `k8s/base/brdgme/`, or `k8s/argocd/` (grepped). These are dead config; the nginx timeouts do
  not apply to the live deployment.
- **Cloudflare (`infra/cloudflare.tf`):**
  - `cloudflare_zone_setting.websockets = "on"` — explicitly required for `/ws` through the
    proxy, with an in-file comment saying so.
  - `cloudflare_zone_setting.ssl = "strict"` (Full (strict)).
  - Rate limiting rule is scoped to `starts_with(http.request.uri.path, "/api/")` only —
    `/ws` is not rate limited at the edge.
  - `cloudflare_bot_management` has `fight_mode = true` / `enable_js = true`, with a comment
    that the documented fallback is `fight_mode = false` if it breaks websockets or login.
  - No Cloudflare-side idle/read timeout is configured in the repo.
- **HTTP protocol versions:** **not determinable from config.** No `appProtocol`,
  `BackendTrafficPolicy`, `h2c`, ALPN, or HTTP/2 setting appears anywhere in `k8s/`,
  `Tiltfile`, `docker-bake.hcl`, `rust/Dockerfile`, or `infra/`. Cloudflare's client-facing
  protocol (HTTP/2, HTTP/3) and Envoy's downstream/upstream protocol selection are defaults
  not expressed in this repo. Only two things are directly readable: the app serves plain
  `axum::serve` over TCP on :3000 (hyper's HTTP/1.1 + h2c auto-negotiation defaults —
  UNKNOWN which is actually used), and a WebSocket handshake succeeding in prod implies the
  client<->edge and edge<->origin legs support it (Cloudflare terminates and re-originates
  WebSockets; for HTTP/2 clients CF handles the `:protocol` extended CONNECT or downgrades —
  INFERRED, not verifiable from this repo).
- **Dev (`Tiltfile`):** under `WEB_IN_CLUSTER`, a `brdgme-dev` Gateway (`gatewayClassName:
  cilium`, HTTP :80) plus an HTTPRoute to `web:3000` on hostname `web.brdgme.lvh.me`, with the
  `cilium-gateway-brdgme-dev` Service NodePort pinned to 31080 to line up with ctlptl's
  hostPort 8080. So dev is `http://web.brdgme.lvh.me:8080` -> plain `ws://`. No WS-specific
  Tilt config. `rust/Dockerfile` and `docker-bake.hcl` contain nothing WS-specific (grepped).
- **`docs/ARCHITECTURE.md`** describes the design: "Clients connect via a single load balancer
  and hold one WebSocket connection to whichever replica they land on. NATS ensures game
  updates published by any replica reach all connected clients for that game." (The last
  clause overstates the implementation — the fan-out is unfiltered, see §5.)

## 7. Tests

- **`rust/web/tests/websocket_hygiene.rs`** — the only dedicated WS integration test file.
  Spins a real `axum::serve` listener on `127.0.0.1:0` via `build_router` (real Postgres via
  `#[sqlx::test]`, real NATS via `NATS_URL`, default `nats://localhost:4222`) and drives it
  with `tokio-tungstenite` 0.30.
  - `live_websocket_survives_idle_past_request_timeout`: asserts the handshake returns
    `101 SWITCHING_PROTOCOLS`; then reads for 32s (> the 30s `REQUEST_TIMEOUT`) asserting the
    connection is neither closed nor dropped, and that **at least one server keepalive `Ping`
    arrives**; then calls `broadcaster.broadcast_game_update(game_id)` and asserts the next
    frame is a `Message::Text` whose JSON `["game_id"]` equals the uuid. Note the client half
    (`_write`) is never used — the test itself sends nothing, corroborating §3.
  - `shutdown_sends_close_frame_to_connected_websockets`: calls `broadcaster.begin_shutdown()`
    and asserts a `Message::Close` arrives within 5s (review finding "ws F55").
- **`rust/web/src/websocket.rs` `mod tests`** —
  `broadcast_publishes_skinny_signal_to_game_subject_only`, currently `#[ignore]`d as flaky
  ("see docs/superpowers/plans/2026-07-07-27-web-simplification.md deferred item 2"). Asserts
  the publish lands on `game.{id}` exactly once with payload `{"game_id": "<uuid>"}`, and that
  nothing is published on `user.>` or `ws.>`.
- **`rust/web/src/game/mod.rs` `mod tests`** —
  `broadcast_and_trigger_publishes_signal_for_missing_game` exercises the shared epilogue.
- **`rust/web/tests/ssr_pages.rs`** and **`rust/web/tests/nats_bot_eventing.rs`** construct a
  `GameBroadcaster` for `AppState` but do not exercise the socket. `ssr_pages.rs`'s
  `tower::ServiceExt::oneshot` harness cannot perform an upgrade (documented in
  `websocket_hygiene.rs`'s module doc).
- **`rust/web/src/app.rs` `mod tests`** — `track_game_seq_retains_seq_on_other_game_updates`,
  `track_game_seq_resets_when_viewed_game_changes` (pure functions, no socket).
- **`rust/web/end2end/`** (Playwright) contains only `tests/helpers.ts` and
  `tests/page-loads.spec.ts`; grepping for `websocket`/`/ws` in that directory returns
  nothing. No browser-level WS coverage exists.
