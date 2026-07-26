# WP-42: realtime visibility predicates and per-connection filter cache

> **Filename is historical.** This file is still
> `WP-42-websocket-auth-and-filtering.md` so existing cross-references resolve.
> The package is **no longer a WebSocket auth package** - the transport half was
> superseded by the SSE pivot on 2026-07-26. What survives is the
> transport-independent visibility work.

**Finding:** ws F59 (predicate half only; the transport half is superseded).
**Decisions:** D-13 ANSWERED 2026-07-25 (option B shape - filter server-side);
**D-44** (COMMIT to SSE, migrate now, ahead of WS hardening); **D-46**
(topology - see `planning/sse-topology-decision.md`).
**Landing order:** **WP-82** (`db.rs` module split) -> **WP-47** -> **WP-42
(this package - predicate work only)** -> **WP-84** (SSE migration).

**What was superseded, and by what.** `specs/WP-84-sse-migration.md` replaces
the entire transport half of this spec:

| Old WP-42 section | Fate |
|---|---|
| §3a pre-upgrade auth dance in `ws_handler` | **SUPERSEDED - do not build.** Replaced by WP-84 §3c. |
| §3b per-connection TTL cache design | **SURVIVES** - transport-independent; WP-84 §3d consumes it by reference. |
| §3c `db.rs` predicates | **SURVIVES** - this is now the bulk of WP-42. |
| §3d Task B (`sub`/`unsub`) | **ELIMINATED - never build.** Struck below. |

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

The realtime feed is anonymous and every connection gets a site-wide firehose.
`ws_handler` (`rust/web/src/websocket.rs`) takes only `WebSocketUpgrade` and
`State<GameBroadcaster>`. `handle_socket` subscribes each connection to the NATS
wildcards `game.>` and `proposal.>` and forwards every payload verbatim, with
no per-connection state and no filtering; all filtering is client-side, in
`use_websocket` (`rust/web/src/websocket_client.rs`). Supporting motivation, not
a separate task - it is **also a load fix**: `use_websocket` bumps
`trigger.last_update` on every frame, and that counter keys the sidebar
`active_games` and `HomePage`'s `public_index` resources, so every site-wide
event forces a server-fn refetch on every connected client.

**This problem is transport-independent.** Whether the pipe is a WebSocket or
an SSE stream, a server-side per-frame predicate is required, and nothing that
exists in `db.rs` today can express it for proposals. That gap is what WP-42
now closes.

## 2. Why it's wrong

- **ws F59 is correct as written.** Verified live: `ws_handler` has no
  `Session`, no `HeaderMap`, no cookie read, no token param, and the upgrade is
  unconditional; `handle_socket`'s `game_sub` / `proposal_sub` arms send the
  payload with no predicate between receive and send.
- **"Identity is already available" holds, and carries over to SSE.** Confirmed
  in `rust/web/src/router.rs::build_router`: `/ws` is registered **before**
  `.layer(session_layer)`, so tower-sessions wraps it and a `Session` extractor
  resolves (`/healthz` is registered after, to bypass it). `FromRef<AppState>
  for PgPool` exists in `rust/web/src/state.rs`. **No router or layer reordering
  is needed** - WP-84 §3a registers `/events` in the same block for the same
  reason. Payloads are skinny (UUIDs only) so the leak is bounded to existence
  and timing - still a leak once D-6 lands.
- **Nothing in the finding was contradicted.** One gap it omits: there is **no
  proposal visibility predicate** in `rust/web/src/db.rs` - see 3c.
- **What changed on 2026-07-26.** The finding's implied fix - authenticate the
  upgrade - is an artefact of the 101 hijack, not of the privacy problem. D-44
  committed to SSE, where identity resolves through ordinary extractors. Writing
  the pre-upgrade dance now means writing it twice and deleting one.

## 3. Required end state

### 3a. `ws_handler` pre-upgrade auth - **SUPERSEDED, DO NOT BUILD**

The previous version of this spec required adding `session:
tower_sessions::Session` and `State(pool): State<PgPool>` to
`websocket.rs::ws_handler` and resolving identity **before** `ws.on_upgrade`,
because after the 101 the connection is hijacked and the session layer's
response-side save pass has already run.

**That entire dance exists only because of the 101 upgrade.** Under D-44 the
transport becomes an ordinary `GET`, so identity resolves anywhere in the
handler with plain extractors and the hand-rolled ordering constraint
disappears. `specs/WP-84-sse-migration.md` **§3c** specifies the replacement -
same two `auth::session` calls, same "never 401 an anonymous connect, degrade to
public-only" rule, same "do not use `get_current_user`" fence, without the
pre-upgrade choreography.

**Do not write it here.** Do not add extractors to `ws_handler`. See §3e for the
consequence (the live `/ws` path stays unfiltered until WP-84 step 2) and why
that is accepted.

### 3b. Per-connection TTL filter cache (SURVIVES - transport-independent)

A reusable, transport-independent cache. It has no WebSocket dependency and
must not acquire one: it is constructed per connection, holds `PgPool` +
`viewer: Option<Uuid>`, and answers "is this id visible" for one frame.

**Design (the open question, resolved): a bounded per-connection TTL cache
keyed by the frame's id - `HashMap<Uuid, (bool, Instant)>`, ~256 entries with
size-capped eviction, 30s TTL, positive and negative cached identically.** No
new crate. Rejected: *DB hit per frame* trades client-refetch amplification for
a Postgres one (a query per frame per connection); *connect-time membership set*
is cheapest but its staleness is unbounded over a connection's lifetime, so a
game joined mid-connection never streams; *positive/negative asymmetry* is
premature - both directions want a short TTL anyway. **Staleness accepted: <=30s
either direction. Failure mode: fail closed** - a `sqlx` error resolves to "not
visible", is `tracing::warn!`d and is **not** cached; do not leak on DB error.
**Escape hatch:** no server push exists, so recovery is TTL expiry, reconnect
(the client already reopens on `visibilitychange` and `online`), and the
client-side `bump_game_update` on own action success - so a user's own join
renders immediately regardless.

Two entry points, one per frame kind: game frames consult
`is_game_visible_to_viewer` (§3c), proposal frames consult
`is_proposal_visible_to_user` (§3c). Keep separate keyspaces - a proposal id and
a game id must never collide in one map.

**Where it lives:** a small module of its own, not inside `websocket.rs`.
Placement is the implementer's call (a private module beside the future
`events.rs`, or a `visibility_cache` module), but it must be importable by
WP-84's `rust/web/src/events.rs` without dragging in any `axum::extract::ws`
type. WP-84 §3d wires it into the `/events` stream verbatim; it is **not**
needed on `/events/public`, which has no per-user predicate.

### 3c. `rust/web/src/db.rs` - one new predicate, no forks (the core of WP-42)

**Game frames:** call WP-47's `is_game_visible_to_viewer(&pool, game_id,
viewer)`; do not write a second encoding of the rule, do not call
`is_game_visible_to_user` directly. **Proposal frames:** add
`is_proposal_visible_to_user(pool, proposal_id, viewer_id) -> Result<bool>`, one
`EXISTS(SELECT 1 FROM game_proposal_players WHERE proposal_id = $1 AND user_id =
$2)`. Proposals have no public form, so an anonymous connection gets no proposal
frames at all, without a query.

**WP-82 lands before this package**, so by WP-42 time `db.rs` is expected to be
`db/` with domain modules and `pub use` re-exports in `db/mod.rs`. Add
`is_proposal_visible_to_user` beside the other proposal functions and re-export
it, matching whatever WP-82 established. Locate the module by domain, not by
remembered path; if the split has not landed, add it to `db.rs` as-is and say
so.

### 3d. ~~Task B - `sub`/`unsub`~~ **ELIMINATED - NEVER BUILD**

Struck 2026-07-26 under D-44. **There is no client->server channel under SSE;
subscription scope lives in the URL** - under the settled two-stream design
(D-48/D-49/D-50) the private, identity-scoped `GET /events` is opened once per
SPA session and never swapped, while the unauthenticated
`GET /events/public?topic=game:<a>&topic=game:<b>` carries the visible topics
and is re-opened as they change. The user intent this was meant to satisfy -
public-game events reaching only clients with that game's page open - is
satisfied by that URL scope instead, at lower cost and with no inbound parsing,
no per-connection `HashSet`, and no client `send` handle.

`specs/WP-84-sse-migration.md` §2 records the same prohibition. The
side-channel-resubscribe alternative was considered and rejected in
`planning/sse-topology-decision.md` §4. Do not resurrect any of it.

### 3e. Does WP-42 touch `rust/web/src/websocket.rs`? **DECIDED: NO.**

**WP-42 is purely `db/` plus the reusable cache module. It makes no edit to
`websocket.rs`. WP-84 does the wiring.**

The reasoning is not "the WS path is throwaway so don't bother" - it is that
**the interim wiring is not actually buildable without the superseded work.**
The filter needs `viewer: Option<Uuid>`, and on the WS path the only way to
obtain it is §3a's pre-upgrade dance. The two available shortcuts both fail:

- Wire the filter with `viewer = None` for everyone: fail-closed then hides
  every private game from its own participants. That is a functional
  regression shipped deliberately, not a security fix.
- Wire the filter and build §3a anyway: that is the superseded work, done in
  full, for a code path WP-84 §7 deletes.

So there is no partial-credit version. WP-84 §5's side-by-side rollout means
`/ws` keeps serving between WP-42 and WP-84 step 2, and **during that window the
firehose persists**. Accepted, explicitly: the leak is bounded to existence and
timing of skinny UUID payloads (§2), it is the status quo rather than a new
regression, and D-44's whole premise is that the window is short because WP-84
follows immediately. **Flag it to the Orchestrator if WP-84 slips**; if the gap
becomes long, the right response is to accelerate WP-84, not to retrofit §3a.

## 4. Non-goals

- **`ws F60`, `ws F61` and `ws F62` remain rows in
  `planning/checklists/T3-B6-outbound-email-websocket.md` and are NOT to be
  done here** - mechanical, independent, must not be done twice. Re-check them
  against WP-84's deletion list before doing them; some may become moot.
- WP-47's own predicate work - consume it, do not author it here.
- The SSE transport itself: routes, handlers, client, shutdown, metrics,
  Cloudflare. All WP-84.
- `ws F55` graceful shutdown, shipped by
  `specs/WP-36-crypto-deploy-hardening.md`: `GameBroadcaster::begin_shutdown`,
  `drain_ws_tasks`, `handle_socket`'s shutdown arm, the ping interval and
  `WsConnectionGuard` all exist live - leave them alone. WP-84 §3g decides
  their fate, not this package.
- The `db.rs` split (`ws F42`) - that is WP-82 and it lands first; per-user NATS
  subjects; the client `(Uuid, seq)` signals and `track_game_seq`.

## 5. Regression test cases

**Recommendation: unit-test the predicate and the cache; write NO new tests
against the WebSocket transport; defer every end-to-end filtering assertion to
WP-84 §8's SSE tests.**

Justification. The previous version of this spec put all its tests in
`rust/web/tests/websocket_hygiene.rs`, a real-listener harness that exists only
because the in-process `oneshot` harness cannot drive a 101 upgrade. **WP-84 §7
deletes that file and its `tokio-tungstenite` dev-dependency outright.** Tests
written there would be deleted within one or two packages, and - per §3e - WP-42
does not change any behaviour observable through `/ws` anyway, so a transport
test would assert nothing new. Meanwhile WP-84 §8 establishes that the existing
`ssr_pages.rs` `oneshot` harness **can** drive an SSE `GET`, so the end-to-end
assertions land there, cheaper and permanently.

The "must have coverage at landing time" concern is real and is met without the
transport: everything WP-42 actually adds is unit-testable directly.

- **`is_proposal_visible_to_user`** - in `db/`'s `#[cfg(test)] mod tests`,
  beside WP-47's `is_game_visible_to_user_*` / `visible_user_ids` tests and
  following their seeding style: a `game_proposal_players` participant is
  visible; a non-participant is not; a nonexistent proposal id is not.
- **The §3b cache** - pure unit tests, no database, against an injected
  fallible lookup fn so the DB is not needed:
  - a repeated id inside the TTL performs exactly **one** lookup (positive and
    negative alike);
  - an entry past 30s is re-looked-up;
  - a lookup error yields `false` **and is not cached** (the next call retries);
  - inserting well past the ~256 cap keeps the map bounded;
  - a game id and a proposal id with the same `Uuid` value do not alias.
- **Do NOT touch `rust/web/tests/websocket_hygiene.rs`.** In particular, leave
  `live_websocket_survives_idle_past_request_timeout` exactly as it is -
  the previous spec's instruction to rework its random-`game_id` broadcast
  assertion is **withdrawn**, because §3e means nothing about that path changes.
  WP-84 §8 replaces that assertion with the anonymous-`GET /events`-returns-200
  case when it deletes the file.
- **Unchanged and must keep passing:** the `user.>` / `ws.>` stay-empty
  assertions in `websocket.rs`'s
  `broadcast_publishes_skinny_signal_to_game_subject_only`. WP-84 §7 keeps that
  test.

**Deferred to WP-84 §8** (listed here so the coverage is traceable, not so it is
duplicated): public-game frame reaches an anonymous stream; private-game frame
does not reach an authenticated non-participant while a game they *are* in does;
proposal frames participant-yes / non-participant-no / anonymous-no.

## 6. Riders

None. `ws F60`-`F62` stay on the checklist.
