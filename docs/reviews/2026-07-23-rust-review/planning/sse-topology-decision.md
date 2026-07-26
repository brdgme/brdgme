# SSE connection topology (D-46) - decision

> # RESOLVED - 2026-07-26. Retained for RATIONALE only.
>
> **D-46 is resolved by measurement, not inference.** Michael ran
> `curl -sI https://brdg.me | head -1` and got `HTTP/2 200`. The browser leg is
> HTTP/2 through the Cloudflare edge, so the ~6-connections-per-origin cap does
> not bite.
>
> **Ruling (D-48): TWO STREAMS** - `GET /events` (private, identity-scoped,
> opened once, never swapped) plus `GET /events/public` (unauthenticated, swapped
> on navigation). The public route's parameter is **not** `?game=<uuid>`: D-50
> replaced it with a repeatable `?topic=game:<a>&topic=game:<b>`. The
> single-stream fallback (Option C) is **deleted** from
> `specs/WP-84-sse-migration.md`.
>
> **This document's conditional recommendation (§5) and its "Michael must
> confirm" list are now HISTORICAL.** Read the body for *why* two streams, not
> for what to build. The authority on what to build is
> `specs/WP-84-sse-migration.md`, backed by D-48/D-49/D-50 in
> `decisions-session3.md`. The body below is deliberately left unedited.
>
> Status of the "Michael must confirm" items:
> 1. HTTP/2 on the browser leg - **ANSWERED: yes, `HTTP/2 200`.** This is what
>    resolves the conditional.
> 2. HTTP/3 - still unconfirmed, still not decision-critical.
> 3. Future SSE uses private/identity-scoped - **ANSWERED by D-49.** Michael's
>    list: private chat, public chat channels (e.g. game-type specific), and a
>    live tournament view including an elimination tree. All hypothetical, none
>    justifies building anything now; the private side stays one stream carrying
>    multiple `event:` types, and no topic machinery beyond `game:` is built.
> 4. Rate limiting - **ANSWERED by D-47 plus WP-84 §6:** establishment only,
>    never duration or bytes, and `/events/public` stays **unmatched** by any
>    rate rule because navigation reopens it and the free tier's fixed 10s period
>    makes that easy to trip.

Answers D-46 only. SSE itself is settled (D-44); `Last-Event-ID` is settled (D-45).
Companion to `ws-to-sse-evaluation.md` and `raw/websocket-inventory.md`.

No line numbers are cited. Read the named function/file; if it does not match, STOP
and report rather than improvising. Anything not read is marked UNKNOWN.

Read for this doc: `infra/cloudflare.tf`, `k8s/base/gateway/gateway.yaml`,
`k8s/base/gateway/httproutes.yaml`, `Tiltfile`, `rust/web/Cargo.toml`,
`rust/web/src/websocket_client.rs`, `rust/web/src/app.rs` (resource sites),
`rust/web/src/proposals.rs` (InvitePage resource site),
`docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md`,
`specs/WP-42-websocket-auth-and-filtering.md`. Grepped `k8s/`, `infra/`, `Tiltfile`,
`docker-bake.hcl`, `rust/Dockerfile` for `http2|http/2|h2c|alpn|http3|appProtocol|
zero_rtt|min_tls`.

**Recommendation up front: two streams, if and only if the HTTP/2 finding below is
confirmed on the browser leg. Otherwise one stream (Option C).** The evidence
points to h2 being confirmable, so two streams is the likely landing point - but
not for the reason Michael gave. The reconnect cost he is worried about is small.
The extensibility argument is the one that carries weight.

---

## 1. The HTTP/2 question

### Browser <-> Cloudflare (the leg that governs the ~6-connection cap)

`infra/cloudflare.tf` sets **exactly two zone settings**: `ssl = "strict"` and
`websockets = "on"`. There is **no** `cloudflare_zone_setting` for `http2`, `http3`,
`zero_rtt`, `min_tls_version`, and **no** `cloudflare_zone_settings_override`
resource anywhere in `infra/`. So the zone's protocol settings are whatever the
dashboard/plan defaults are, and those defaults are **not readable from this repo**.

The repo does contain indirect evidence. `docs/superpowers/plans/
2026-07-10-28-wp4-cloudflare-pre-golive.md` - the plan that created this exact zone -
specifies three verification steps whose stated expected output is
`HTTP/2 200`, `HTTP/2 200` and `HTTP/2 429` from `curl -sI https://beta.brdg.me...`
through the proxy (`server: cloudflare`, `cf-ray:` present). That is a
TLS+ALPN negotiation against the CF edge for this zone. Caveat: those steps are
written as **expectations, and the checkboxes are not ticked** (only 3 `- [x]` in
the whole file), so the repo records an expectation, not a recorded observation.

Verdict: **h2 on the browser leg is very likely but NOT proven from repo state.**
Michael must confirm - it is a 5-second check, either:
- `curl -sI https://brdg.me | head -1` (expect `HTTP/2 200`), or
- the CF dashboard: Speed -> Optimization -> Protocol Optimization (HTTP/2, HTTP/3).

I have **not** read Cloudflare's documentation and will not assert what the free-plan
defaults are - **UNKNOWN from repo evidence.**

### Cloudflare <-> origin

`ssl = "strict"`, so CF re-originates TLS to the Cilium/Envoy Gateway
(`k8s/base/gateway/gateway.yaml`, listener `web` :443 HTTPS with `brdg-me-tls`).
Envoy could in principle negotiate h2 to the pod, but `rust/web/Cargo.toml` still
declares `axum = { version = "0.8.9", features = ["ws", "macros"] }` and axum's
`http2` feature is enabled nowhere in the workspace, so the origin is
HTTP/1.1-only (as the prior evaluation established from the feature graph;
`leptos_axum 0.8.10` was read there and does not enable it). **This leg does not
govern the browser cap** - it just means every browser stream becomes a separate
h1 connection from Envoy to a pod. At 2 replicas and the current user count that is
irrelevant; it would only matter at a scale brdgme is nowhere near.

### Dev

Both dev modes are **plain HTTP, therefore HTTP/1.1**:
- Default Tilt mode runs `cargo leptos watch` locally on port 3000 - the browser
  hits `http://localhost:3000`. No TLS, so no ALPN; and axum has no `http2`
  feature, so h2c-with-prior-knowledge is not available either.
- `WEB_IN_CLUSTER=1` serves `http://web.brdgme.lvh.me:8080` via a `brdgme-dev`
  Gateway whose only listener is `port: 80, protocol: HTTP`.

So **dev is permanently h1 and permanently subject to the ~6 cap**, regardless of
what prod does. Any multi-stream design must be sized for dev, not prod.

**Plainly:** under h2 the ~6-per-origin cap dissolves (streams are multiplexed on
one connection; typical limit ~100 concurrent streams) and a second SSE stream is
nearly free. Under h1 each stream permanently occupies one of ~6 slots shared with
asset loads and Leptos server-fn POSTs.

## 2. What an Option C reconnect actually costs

Client state on the connection: **none.** D-45 forbids `id:`, and the payloads are
already stateless cache-invalidation pings (`{"game_id":...}` / `{"proposal_id":...}`).
Reconnect means "bump the triggers and refetch what is visible".

What refetches. The trigger fan-out is small and was read directly:
- `WebSocketTrigger.last_update` keys exactly two resources: `active_games`
  (`get_sidebar_games`) in `app.rs::App`, and `public_index` (`get_public_index`)
  in `app.rs::HomePage`.
- `seq_for_this_game` keys `game_data` (`get_game_details`) and `logs`
  (`get_game_logs`) in `app.rs::GamePage`.
- `seq_for_this_proposal` keys `proposal_data` (`get_proposal`) in
  `proposals.rs::InvitePage`.

The decisive detail: **the only thing that triggers an Option C reopen is navigating
to a different game page - and that navigation already refetches `game_data`,
`logs` and `mark_read` on its own.** The incremental cost of the reconnect is
`get_sidebar_games`, plus `get_public_index` if the home page is mounted. That is
**one extra server-fn POST on a navigation that was already issuing three.**

Server cost per reopen: one session load (the tower-sessions Postgres layer already
runs on every request including `/ws` today), the WP-42 auth pair
(`get_user_from_session` + `validate_session_token`), two NATS Core wildcard
subscribes (`game.>`, `proposal.>`), and **discarding the per-connection TTL cache**
that WP-42 §3b specifies (bounded, 30s TTL, positive and negative entries). Losing
that cache costs a handful of visibility-predicate `sqlx` queries that then re-warm.
All of it is small and none of it is unbounded.

Frequency: navigation between game pages only. Not per keystroke, not per move.

**Honest assessment: Option C's reconnect cost is small, not zero, and is not by
itself a reason to reject Option C.** Two real irritants remain, both minor:
stream churn under rapid clicking through the sidebar (needs a debounce on the URL
signal), and a brief window per navigation where no stream is open.

## 3. One stream vs two, on the merits

### One stream - Option C, `GET /events?game=<uuid>`

- URL/auth: `/events` + optional `?game=`/`?proposal=`. Ordinary `GET`; `Session`
  and `State<PgPool>` extract normally. Anonymous allowed, degraded to publicly
  visible games.
- Reconnect: browser-native; scope lives in the URL so it survives reconnects for
  free. leptos-use 0.19.0's `use_event_source` takes `impl Into<Signal<String>>`
  and re-opens on URL change (verified in the prior evaluation).
- Fan-out: two wildcard subscribes per connection, per-frame predicate + TTL cache.
- **Connections held: 1.**
- Extensibility: any new feed either widens this stream's predicate or adds another
  query param. Chat would ride the same connection and therefore drop on every
  game navigation.

### Two streams - `GET /events` (private) + `GET /events/public?game=<uuid>`

- `/events`: identity-scoped, no query param, **opened once per SPA session and
  never re-opened for navigation**.
- `/events/public?game=<uuid>`: swapped on navigation. **Needs no auth at all** -
  and this is a genuinely strong point that the prior evaluation understated: a
  publicly-visible game's id and the fact that it changed are already public
  information, so this stream leaks nothing by design. No session extractor, no
  per-user predicate, no TTL cache, no `sqlx` on the hot path beyond a
  "is this game public" check.
- Reconnect: independent. The private stream is untouched by navigation.
- Fan-out: two connections x two wildcard subscribes = 4 NATS subscribes per client
  instead of 2. NATS Core wildcard subscribes are cheap; this is not the constraint.
- **Connections held: 2.**
- Duplicate delivery for a public game the viewer also participates in: harmless,
  the `(id, seq)` bump already tolerates redundant refetches by design (see
  `bump_game_update`'s doc comment).

### How many connections does a brdgme page actually hold?

Held-open, continuously: **1 today** (the WebSocket). Everything else is bursty -
document, `/pkg/` JS glue, `/pkg/` wasm, CSS, and Leptos server-fn POSTs, which
open, complete and release. Under h1 with a ~6 budget: 1 SSE leaves ~5 for bursts;
2 SSE leaves ~4. Both are survivable. **3+ held streams in dev would be genuinely
dangerous** - that is where stalls appear that never reproduce in prod.

## 4. Two other shapes considered

**Side-channel resubscribe (`POST /events/<connection-id>/subscribe`).**
Rejected. It reintroduces a client->server application channel, which is precisely
the property whose absence made SSE attractive (inventory §3: today's reverse
channel carries only pongs and close frames). It also requires a
connection-id registry that must be **replica-affine** - with `replicas: 2` and no
session affinity configured in `k8s/base/gateway/httproutes.yaml`, the POST can land
on the pod that does not hold the stream. Solving that means either sticky routing
or a NATS control subject. That is strictly more machinery than WP-42 Task B, which
SSE was supposed to delete. Not worth it.

**One stream carrying all publicly-visible games, no per-game param.**
Feasibility is not theoretical: **this is essentially what production does today.**
Every `handle_socket` subscribes `game.>` and `proposal.>` with **no filtering at
all** (inventory §5), so every connected client already receives every game and
proposal signal in the system, and it works. Restricting that firehose to
publicly-visible games strictly reduces volume.

Actual publish volume is **UNKNOWN** - there is no counter or metric for NATS
publishes in the repo (the only WS-related metric is the `ws_connections` gauge).
What governs it: the 15 publish call sites (inventory §5), i.e. roughly one publish
per player move, per proposal mutation and per bot turn, multiplied by concurrent
active games. For a play-by-email-paced board game site this is small; if brdgme
grew real-time-heavy games it would not be.

This shape is attractive because it removes the swap entirely - **both** streams
become long-lived and navigation reopens nothing. Its cost is bandwidth to every
idle client and a slightly noisier client-side filter (already implemented:
`track_game_seq` discards ids for other games). **It is a viable simplification of
the two-stream option and should be the fallback if per-game scoping proves fiddly**,
but it should not be the initial design, because it scales with total site activity
rather than with what the user is looking at.

## 5. Recommendation

**If h2 is confirmed on the browser leg (expected): two streams.**
- `GET /events` - private, identity-scoped, opened once, never swapped.
- `GET /events/public?game=<uuid>` - unauthenticated, swapped on navigation.

The reasoning is **not** the reconnect cost, which section 2 shows is roughly one
extra server-fn call on a navigation that was already making several. It is:
1. `/events/public` needs **no auth and no visibility predicate at all**, because
   public game ids are public. That is a real reduction in security-critical
   surface, not just a separation of concerns.
2. The private stream stops being coupled to page navigation, which is the correct
   architecture for anything later added to it - chat, notifications, presence.
   Michael's extensibility instinct is right even though his stated reason is weak.
3. Under h2 the second stream costs essentially nothing.

**If h2 is NOT confirmed on the browser leg: one stream (Option C).** Two permanently
held h1 connections out of ~6, in prod *and* dev, is not worth buying a
once-per-navigation refetch and future optionality. Revisit if h2 is later enabled.

**Either way, in dev, expect h1.** Two streams leaves ~4 burst slots, which is
adequate but must be watched. **Do not let the design grow to 3 held streams**
without re-opening this decision - chat must ride `/events`, not a third stream.

Everything else from the prior evaluation is unaffected: `KeepAlive` (D-47's
heartbeat), no `id:` (D-45), `ReconnectLimit::Infinite` on the client, WP-42's
predicate work reused verbatim on `/events` (and **not needed** on `/events/public`).

## Michael must confirm

1. **`curl -sI https://brdg.me | head -1`** - does the CF edge answer `HTTP/2`?
   This single output decides between the two recommendations above.
2. Whether HTTP/3 is also on (dashboard, Speed -> Protocol Optimization). Not
   decision-critical; h3 only strengthens the two-stream case.
3. That the intended future uses of SSE (chat, notifications) are **private/
   identity-scoped**. If any future feed is a third independently-swapped public
   feed, say so now - it changes the answer.
4. Rate limiting: D-47 rules connection-establishment-only. With two streams a
   navigation-heavy session opens `/events/public` repeatedly. Any `/events` rate
   rule must be sized for that, or scoped to `/events` and not `/events/public`.
