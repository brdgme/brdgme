# Verification: web-server infra findings F52-F67

Verifier: independent Worker, snapshot f8763a5 at
/home/beefsack/Development/brdgme-review-snapshot. All line refs are snapshot
lines. No code changes made.

## F52 (major) Session cookies lack Secure flag in prod; SECURE_COOKIE never set

Verdict: CONFIRMED

Evidence:
- rust/web/src/auth/session.rs:32-36:
  ```rust
  let secure = std::env::var("SECURE_COOKIE")
      .map(|v| v == "true")
      .unwrap_or(false);
  SessionManagerLayer::new(store)
      .with_secure(secure)
  ```
- Snapshot-wide grep for `SECURE_COOKIE` returns exactly two hits: the read
  above and docs/superpowers/plans/2026-07-02-07-pre-cutover-fixes.md:26 (a
  completed-plan doc). The "exactly two places" claim is accurate.
- k8s/base/web/deployment.yaml env block (lines ~40-48) sets only NATS_URL and
  PUBLIC_BASE_URL plus three secretRefs (postgres-config, email-config,
  database-encryption-key); no SECURE_COOKIE. Neither dev/ nor prod/
  kustomizations patch web env. rust/web/.env.template does not mention it.
- Therefore prod pods run `with_secure(false)`: no `Secure` attribute on the
  session cookie.

Severity: major stands. Real session-token exposure over any plaintext HTTP
path to the domain; not critical because HSTS/redirects at the edge may
mitigate in practice (unverified here).

Recommendation validity: mostly fine, one caveat. `k8s/dev/kustomization.yaml`
includes `../base/web` directly, so setting `SECURE_COOKIE=true` in the BASE
deployment also applies to the dev cluster; if dev is accessed over plain HTTP
on a non-localhost host, login breaks (browsers do accept Secure cookies on
http://localhost, so localhost/Tilt port-forward would still work). The
recommendation's second alternative (default secure in code, explicit
`SECURE_COOKIE=false` opt-out for local dev) is the safer form; if the env-var
route is taken, a prod overlay patch is cleaner than base.

## F53 (major) Bot command consumer task unsupervised

Verdict: CONFIRMED

Evidence:
- rust/web/src/main.rs:55-74: `tokio::spawn` of an async block that runs
  `run_bot_command_consumer` and on `Err` only does
  `tracing::error!("bot.command consumer exited: {}", e);`. The JoinHandle is
  not bound (spawn result discarded), so a panic in the task is also silent.
  No restart loop, no select against `axum::serve`.
- rust/web/src/game/mod.rs:322-325: the consumer also returns `Ok(())` when
  the `messages()` stream ends - that exit path is not even logged as an
  error at the spawn site (the `if let Err` arm never fires).
- rust/web/src/router.rs:203-205: `async fn healthz() -> &'static str { "OK" }`
  - static, deliberately DB-independent (comment at :157-162), covers nothing
  about the consumer. k8s liveness/readiness both probe /healthz
  (k8s/base/web/deployment.yaml:49-60), so k8s sees a healthy pod while bots
  are dead.
- Cross-ref: web-domain findings independently reported the same defect
  ("bot.command consumer is spawned once and never restarted if it exits").

Severity: major stands (silent permanent bot outage until pod restart).

Recommendation validity: sound. All three options (restart loop with backoff,
select! against serve so the process dies and k8s restarts it, deeper health
check) are workable; the select!/abort option is the simplest and matches the
existing k8s probe setup.

## F54 (minor) No rustls CryptoProvider installed in web main

Verdict: CONFIRMED (with the finding's own stated uncertainty intact)

Evidence:
- `grep -rn "CryptoProvider\|install_default" rust/web/src` - zero hits; web's
  main installs no provider.
- rust/Cargo.lock contains both `aws-lc-rs` (line 519) and `ring` (line 5070).
- docs/CODING.md:408-423 documents the project rule verbatim: workspace
  enables both backends; reqwest's `rustls` feature enables aws-lc-rs while
  sqlx/kube/async-nats defaults enable ring; rustls 0.23 panics at first use
  when both are enabled and no default is installed; the prescribed fix is
  `rustls::crypto::aws_lc_rs::default_provider().install_default()` in main.
- web/Cargo.toml has `reqwest ... features = ["json", "form", "rustls"]`
  (line 70), `sqlx ... runtime-tokio-rustls` (line 28), `async-nats` (line 79)
  - i.e. the exact combination CODING.md warns about. Whether the resolved
  graph actually enables both provider features for the web binary remains
  UNVERIFIABLE without `cargo tree` (no builds allowed), as the finding says.
  Prod evidently works today, so either one provider is enabled or every
  consumer selects explicitly; nothing guards against a bump changing that.

Severity: minor stands (latent, defensive).

Recommendation validity: valid and matches the project's own documented rule;
`.install_default().ok()` is the documented always-safe idiom. The alternative
(record a `cargo tree` answer in CODING.md) is weaker - it rots on the next
dependency bump - but harmless.

## F55 (minor) Graceful shutdown does not cover WS/background tasks

Verdict: CONFIRMED

Evidence:
- rust/web/src/main.rs:104-110: `axum::serve(...).with_graceful_shutdown(shutdown_signal())`.
- rust/web/src/router.rs:26-31 (comment) and websocket.rs:86:
  `ws.on_upgrade(...)` hands the socket to a detached spawned task; nothing
  tracks those tasks or sends close frames on shutdown.
- main.rs:55-74 (bot consumer) and :75-80 (email sweeps) receive no shutdown
  signal; the consumer leaves un-acked messages to redeliver (safe), sweeps
  are periodic and idempotent-looking.

Severity: minor stands; the finding itself says "acceptable as-is for a beta"
- arguably a nit, but every deploy hard-dropping all WS clients justifies
minor.

Recommendation validity: fine (CancellationToken/TaskTracker is the standard
pattern; explicitly optional).

## F56 (minor) Messages exhausting max_deliver=3 strand silently

Verdict: CONFIRMED, severity stays minor (uncertainty resolved)

Evidence:
- rust/web/src/nats.rs:52-94: WorkQueue retention stream (:57), both durable
  pull consumers with `AckPolicy::Explicit`, `ack_wait = 5*60s` (:63),
  `max_deliver = 3` (:64). No advisory subscription anywhere; no DLQ.
- Cross-check resolved by reading the consumer directly
  (rust/web/src/game/mod.rs:265-321): `term()`/`nak()` are never called.
  Poison-message classes ARE removed: unparseable payload -> `ack()` (:277),
  `UserError` (game rejected, never succeeds on redelivery) -> `ack()`
  (:304-313), `Ok`/`Conflict` -> `ack()` (:299-302). Only
  `ExecuteCommandError::Other` (transient) is left unacked for redelivery
  (:315-320).
- So the stranding window is exactly: a message failing with `Other` on all 3
  deliveries (e.g. game service down > ~2 x ack_wait). It then sits in the
  WorkQueue stream forever (deleted only on ack), the bot never moves in that
  game, MAX_DELIVERIES advisory unhandled, no metric.
- The finding's downgrade/upgrade condition: the consumer does NOT term()
  poison messages, but it ack()s them - functionally equivalent on a
  WorkQueue stream (ack deletes the message). So neither the "downgrade to
  nit" (that condition was about poison looping, which is handled) nor the
  "upgrade to major" (poison is not left to strand) branch strictly fires:
  minor is right. The web-domain unit reached the identical conclusion
  ("CONFIRMED and stays minor").

Recommendation validity: sound (term + compensate, or advisory listener /
DLQ, or stream max_age). web-domain's variant (use `message.info()`
num_delivered and term at ceiling with metric) is the most concrete.

## F57 (minor) get_or_create_stream/consumer never reconcile config drift

Verdict: CONFIRMED

Evidence:
- rust/web/src/nats.rs:53-61 `get_or_create_stream` and :66-94
  `get_or_create_consumer` (x2). async-nats `get_or_create_*` returns the
  existing object untouched when present; no compare/warn after retrieval,
  so editing ack_wait/max_deliver/retention/subjects in code is a silent
  no-op against an existing deployment.

Severity: minor stands.

Recommendation validity: fine. Note per NATS semantics some stream config
changes CAN be applied via update while consumer config generally requires
delete/recreate - the recommendation's "document that consumer config
changes require manual deletion/recreation" is accurate.

## F58 (minor) ack_wait=5min may be shorter than processing

Verdict: ADJUSTED - downgrade to nit; the uncertainty is resolved and the
risk is essentially closed

Evidence:
- nats.rs:63 `ack_wait = Duration::from_secs(5 * 60)`, confirmed.
- Consumer cadence resolved: rust/web/src/game/mod.rs processes one message
  fully then acks; no `in_progress()` anywhere. Processing =
  `handle_bot_command_event` = DB reads/writes + `execute_command`, using the
  shared reqwest client built in main.rs:32-36 with a hard 10s total timeout
  and 5s connect timeout; retry budgets are bounded (MAX_TURN_ATTEMPTS=3).
  Exceeding 5 minutes is implausible with those bounds.
- The web-domain verification note says the same: "the ack_wait concern is
  closed (bounded processing, though unguarded by in_progress pings)".

Severity: nit (was minor). The residual value is the unguarded-by-in_progress
observation.

Recommendation validity: the recommended cross-check has been performed (this
document); no code change needed.

## F59 (minor) /ws unauthenticated site-wide firehose

Verdict: CONFIRMED

Evidence:
- rust/web/src/router.rs:142: `.route("/ws", axum::routing::get(crate::websocket::ws_handler))`
  - registered BEFORE `.layer(session_layer)` (:155) so the session layer does
  apply to it, but the handler ignores it.
- rust/web/src/websocket.rs:82-87: `ws_handler(ws: WebSocketUpgrade,
  State(broadcaster))` - no Session extractor, no auth check.
- websocket.rs:112-125: every connection subscribes core NATS `game.>` and
  `proposal.>` - the full site-wide signal stream, forwarded verbatim
  (:134-159). Payloads are skinny `{game_id}` / `{proposal_id}` UUIDs
  (:39-58), so metadata leak + O(connections x site update rate) fan-out, as
  stated. Per-game publish subjects exist (`game.{id}`), so per-connection
  filtering is feasible.

Severity: minor stands (metadata-only leak; scalability concern is real but
not yet pressing).

Recommendation validity: sound; "document if accepted, else
subscribe-per-id and/or require a session" are both reasonable.

## F60 (minor) Client open() on visibilitychange/online tears down healthy sockets

Verdict: CONFIRMED (now source-verified locally, not just external-basis)

Evidence:
- rust/web/src/websocket_client.rs:70-85: both the `visibilitychange`
  (-> Visible) listener and the `online` listener call `open()`
  unconditionally; `ready_state` is destructured away (`ready_state: _`,
  :52).
- Vendored leptos-use 0.19.0 source verified at
  ~/.cargo/registry/src/index.crates.io-*/leptos-use-0.19.0/src/use_websocket.rs:
  - `open` (:695-701) resets reconnect_times and calls `connect` with NO
    ready-state guard.
  - `connect` (:474-486): `if let Some(web_socket) = ws.get_value() {
    let _ = web_socket.close(); }` then creates a fresh WebSocket - i.e. it
    unconditionally closes a healthy open socket.
  - The old socket's `onclose` (:662-684) calls `reconnect()`; `reconnect`
    (:445-460) fires when the CURRENT `ws` ready_state != OPEN - by then `ws`
    holds the brand-new CONNECTING socket, so a gratuitous second reconnect
    is scheduled after the default ~3s interval, which in turn `close()`s the
    by-then-open replacement. The "~1-2 gratuitous reconnects per refocus"
    claim is accurate.
- Server cost per reconnect: two new NATS subscriptions (websocket.rs:112-125).

Severity: minor stands.

Recommendation validity: valid with one implementation note - gate on
`ready_state`, but read it with `.get_untracked()` inside the event
listeners (they are not reactive contexts); guarding only when
`ConnectionReadyState::Closed` also skips reopening a socket stuck in
Connecting, which is the desired behavior since leptos-use's own reconnect
timer owns that state.

## F61 (nit) WS inbound limits at tungstenite defaults

Verdict: CONFIRMED

Evidence: rust/web/src/websocket.rs:86 - `ws.on_upgrade(...)` with no
`.max_message_size()` / `.max_frame_size()`; axum's defaults are the
tungstenite defaults (~64 MiB message / 16 MiB frame). Inbound frames are
drained and discarded (:167-172), so the only need is close/pong - small
explicit limits are free hardening. Combined with F59 (anonymous
connections), each connection can force buffering up to the max.

Severity: nit stands.

Recommendation validity: fine (`ws.max_message_size(4*1024).max_frame_size(4*1024)`
is a real axum builder API).

## F62 (nit) No dead-connection detection beyond send failure

Verdict: CONFIRMED

Evidence: rust/web/src/websocket.rs:127-172 - Ping sent every 30s (:128-130,
:160-164); inbound (including Pongs) is drained without recording anything
(:167-172); no last-pong tracking, no read deadline. A half-open client keeps
the task, the `ws_connections` gauge increment, and two NATS subscriptions
until TCP-level failure surfaces as a send error. Bounded and self-healing,
as stated.

Severity: nit stands.

Recommendation validity: fine.

## F63 (nit) Unbounded file read in import_game

Verdict: CONFIRMED

Evidence: rust/web/src/bin/import_game.rs:20 -
`std::fs::read_to_string(&path)` with no size guard. File header (:1-6)
confirms dev-only, never deployed. Otherwise clean as the findings say
(usage() exits 2, anyhow context includes path, prints URL).

Severity: nit stands ("none required" recommendation is appropriate).

## F64 (minor) gloo-net dependency unused

Verdict: CONFIRMED

Evidence:
- rust/web/Cargo.toml:75: `gloo-net = { version = "0.7", features = ["websocket"] }`,
  non-optional (so it is in the hydrate/WASM graph - correct, since only
  `optional = true` deps are excluded from default resolution and the
  hydrate feature does not gate it).
- `grep -rn "gloo_net" web/src web/tests` - zero hits.
- `gloo-timers` (line 76) IS used (websocket_client-adjacent code /
  elsewhere) - the finding's "keep it" aside was not re-verified in depth
  but grep shows gloo_timers usage exists in src.

Severity: minor stands (dead WASM-bundle weight).

Recommendation validity: fine (delete the line).

## F65 (minor) tokio net/time features used but not declared

Verdict: CONFIRMED

Evidence:
- rust/web/Cargo.toml:24: `tokio = { version = "1", features =
  ["rt-multi-thread", "macros", "signal"], optional = true }` - no "net", no
  "time".
- Direct uses: `tokio::net::TcpListener` at main.rs:103 and :184 (plus
  tests/game mod tests); `tokio::time::interval`/`MissedTickBehavior` at
  websocket.rs:128-129 and email/sweep.rs:222-223, :249-250, :312. Compiles
  only via feature unification from transitive deps (axum enables tokio/net,
  etc.).

Severity: minor stands.

Recommendation validity: fine (add "net", "time").

## F66 (nit) futures-util non-optional but only ssr/test

Verdict: CONFIRMED, with a caveat on the recommendation

Evidence:
- rust/web/Cargo.toml:74: `futures-util = "0.3.32"`, unconditional.
- All uses are ssr-gated or test code: websocket.rs:27 (inside
  `#[cfg(feature = "ssr")] mod ssr`), game/mod.rs:258 and :1079 (fn/test
  under `#[cfg(feature = "ssr")]`), auth/server.rs:1162,1204 (test code in
  an ssr-gated module), tests/websocket_hygiene.rs:14 and
  tests/nats_bot_eventing.rs:19 (integration tests).

Severity: nit stands.

Recommendation validity: CAVEAT. Making it `optional = true` +
`dep:futures-util` under `ssr` is correct for the library, but the
integration tests (tests/websocket_hygiene.rs, tests/nats_bot_eventing.rs)
`use futures_util` at the top level with no cfg gate; `cargo test` without
`--features ssr` would then fail to compile those files (top-level `use` is
not feature-gated even if the test bodies are). Fix requires either running
tests only with ssr (may already be the project convention), adding
`futures-util` to [dev-dependencies], or `required-features` via [[test]]
stanzas. Flag this so the fix does not break the test build.

## F67 (nit) Dependency currency spot-check

Verdict: UNVERIFIABLE (external basis) for the version-currency claims;
in-repo parts CONFIRMED

Evidence (in-repo, verified):
- Declared versions match the finding: axum 0.8.9 (:20), tower-http 0.7
  (:41), sentry 0.48 (:86), resend-rs 0.28 (:46), mrml 6.0.1 (:49), reqwest
  0.13 (:70), leptos-use 0.19 (:80), async-nats 0.49.1 (:79), svix 1.98
  (:50), sqlx 0.8 (:28), tower-sessions 0.14.0 (:39), wasm-bindgen =0.2.121
  pin (:25).
- The "documented intentional" holdback claim: the wasm-bindgen pin and
  petname `=3.1.0` pin are visible in the manifest; the sqlx/tower-sessions
  holdback documentation was not independently located in this pass but is a
  low-stakes claim.
- Whether async-nats 0.50.0 / svix 1.99.1 were current on crates.io as of
  2026-07-24 cannot be checked offline - external-basis, taken on trust.

Severity: nit stands.

Recommendation validity: fine ("bump at the next dependency pass, check the
JetStream changelog first" is appropriately cautious).

## Summary of severity changes

- F58: minor -> nit (consumer acks fast, 10s HTTP timeout bounds processing;
  in_progress absence is the only residue).
- All others stand as filed.

## Recommendation flags

- F52: setting SECURE_COOKIE=true in base/ also hits the dev overlay (dev
  kustomization includes ../base/web); prefer the code-default-secure
  alternative or a prod overlay patch.
- F60: gate open() using ready_state with get_untracked() inside the event
  listeners; only open when Closed.
- F66: making futures-util optional breaks `cargo test` without --features
  ssr unless the two integration tests' top-level `use futures_util` lines
  are handled (dev-dependency or [[test]] required-features).
