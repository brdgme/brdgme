# Realtime event-delivery topology (SSE)

**DECIDED 2026-07-26:** the live-update transport is Server-Sent Events, not
WebSockets. Two streams - a private identity-scoped stream and an
unauthenticated public stream - with no replay and a hard cap of two held
streams per client.

## Context

brdgme's live updates are server -> client only. The WebSocket reverse channel
carries no application traffic: the server discards inbound frames and the
client never binds a send handle. WebSocket's defining capability is unused,
and its 101-upgrade hijack is exactly what forces hand-rolled pre-upgrade auth
on the `/ws` route. SSE is an ordinary `GET`: session and state extract
normally, and an invalid session is a normal response rather than a
pre-upgrade dance.

## Decision 1 - SSE now, not after hardening WebSockets

Migrate to SSE now, ahead of any WebSocket hardening. Hardening the
upgrade-hijack machinery and then deleting it is wasted effort. The transport
choice is also forward-looking: a likely future web framework (the Tokio
team's Topcoat) plans to support SSE and not WebSockets, and the current
framework (Leptos) carries bus-factor risk - so the product should not build
on a transport a future framework will not support. The earlier `/ws`
hardening design is historical, not current work.

## Decision 2 - no `Last-Event-ID` replay

Do not emit `id:`. Reconnect means "refetch everything visible". The fan-out
is NATS Core, which has no replay buffer, so honouring `Last-Event-ID` would
be a promise the server cannot keep. The payloads are cache-invalidation
pings, so refetch-on-reconnect is the correct behaviour and keeps the
implementation simple.

## Decision 3 - rate-limit establishment only, plus a server heartbeat

Rate-limit connection ESTABLISHMENT for `/events`; never rate-limit by stream
duration or bytes streamed. The connection must stay open as long as the page
is open. A server-side heartbeat (an SSE comment line) runs at an interval
comfortably below any proxy idle timeout so an idle stream is never reaped.
The public stream is deliberately left UNMATCHED by any edge rate rule,
because navigation reopens it constantly and a fixed short window trips too
easily. The implementing spec must verify the actual edge configuration and
proxy idle timeout rather than assuming them.

## Decision 4 - two streams, never three

The browser leg is HTTP/2 through the edge (measured: `HTTP/2 200`), so the
HTTP/1.1 ~6-connections-per-origin cap does not bite and a second stream is
nearly free. Two streams:

- `GET /events` - private, identity-scoped, opened once per SPA session and
  NEVER swapped on navigation.
- `GET /events/public?topic=game:<uuid>` - unauthenticated, swapped as the
  visible public game changes. It needs no auth and no visibility predicate,
  because public game ids are already public. That surface reduction, not the
  reconnect cost, is the load-bearing argument.

Hard cap: a client never holds three streams. Future SSE uses (chat,
notifications) ride the existing private stream. A third independently swapped
public feed would reopen this decision. Dev is permanently HTTP/1.1 (both
Tilt modes are plain HTTP), so any future stream-count increase must be
re-checked against dev, not just production.

## Decision 5 - build nothing extra

Build exactly the two streams. No topic machinery beyond `game:`, no
multiplexing layer, no channel registry, no subscription protocol. The only
thing to get right now is avoiding a shape that would need a redesign later.
On the private stream, meaningful `event:` field names make "one stream,
multiple message types" work, so keep event names meaningful from day one.

## Decision 6 - the public topic param is repeatable

`GET /events/public?topic=game:<a>&topic=game:<b>` - the same key repeated,
parsed into a collection from day one. Accept N `game:` topics; reject every
other topic kind, and reject malformed topics with an error rather than
silently ignoring them, so a client bug surfaces immediately. No `[]` suffix -
repeated keys already carry array-ness. Topic KINDS (`tournament:`, `chat:`)
are speculative and rejected; topic COUNT (several games at once) is a
plausible near-term product move, and parsing into a collection now avoids
baking a single-game assumption into the subscription bookkeeping and fan-out
path, which is the expensive part to undo.

## Decision 7 - public topic cap = 16

A public stream accepts at most 16 topics. Over the cap is a 400, not a
truncation, so a UI asking for too many fails visibly rather than silently
dropping topics. Because `/events/public` is unauthenticated and deliberately
unmatched by any rate rule, the cap is the only bound on what one connection
can ask the server to watch. Raising a cap later is backward compatible;
lowering one breaks existing clients - so 16 is the safe direction to be wrong
in.

## Alternatives considered

- **One stream, subscription in the URL (`GET /events?game=<uuid>`).**
  Rejected: changing the query param reopens the single connection, so the
  private/player-data stream drops on every public-game navigation even though
  the private subscription is unchanged.
- **`Last-Event-ID` replay.** Rejected: NATS Core has no replay buffer; it
  would require a new backing store for a promise the payloads do not need.
- **Side-channel resubscribe (`POST /events/<id>/subscribe`).** Rejected: it
  reintroduces a client -> server channel and needs replica-affine routing -
  strictly more machinery than SSE was meant to delete.
