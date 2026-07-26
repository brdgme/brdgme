# Decisions - planning session 3 (2026-07-26)

New rulings from Michael this session. These extend `decisions-ANSWERED.md`
(D-01..D-34) and the D-numbers referenced in `ORCHESTRATOR-HANDOVER.md`.
Fold into `decisions-ANSWERED.md` when convenient.

## D-41 - Delete the per-game `_fuzz` and `_repl` binaries

**Ruling: DELETE, but verify first.**

`rust/tools/fuzz` and `rust/tools/repl` are already generic out-of-process
drivers that shell out to a game's `_cli` binary. That makes the 27 `_fuzz` and
27 `_repl` per-game bins redundant - **54 deletable files**.

Michael's rationale: it is simpler, and the out-of-process boundary is what
would make non-Rust game implementations viable again in future.

Required before deleting: confirm by reading and searching that nothing outside
`rust/tools/fuzz` and `rust/tools/repl` invokes the per-game `_fuzz`/`_repl`
binaries. Known: `rust/Dockerfile` copies only `target/release/<snake>_http`.
Also check `docker-bake.hcl`, `Tiltfile`, k8s manifests, CI config, `justfile` /
`Makefile` equivalents, docs, and any test harness. **If anything does depend on
them, STOP and report - do not delete.**

Consequence for WP-73: `brdgme_game_bin` then needs only **two** generic entry
points (`cli_main`, `http_main`), not four. `fuzz_main` and `repl_main` are
dropped from the spec.

## D-43 - SUPERSEDES D-41 for fuzz: throughput beats simplicity

**Ruling: the `_fuzz` deletion is REVERSED pending a performance evaluation.
D-41 stands for `_repl` only.**

The dependency sweep was clean, but it surfaced an accepted cost that Michael
has now rejected: `LocalRequester` spawns **one child process per API request**,
so out-of-process fuzzing is materially slower than the in-process `fuzz_gamer`
path D-41 would have deleted.

Michael's rationale, verbatim: "we want this to be as fast as possible to
maximise the value of the fuzzer, the value of the fuzzer is basically directly
correlated by how fast it can run and how many games it can pump through. I
think we need to consider which approach is the absolute fastest over
simplification or portability."

**Selection criterion for fuzzing is raw throughput.** Simplicity and the
non-Rust-game portability argument are explicitly subordinate for this one
concern. They still apply to `_repl`, which is interactive and has no throughput
requirement.

Open: whether the status quo (per-game `_fuzz` bin, in-process) is actually the
fastest available option, or whether something faster exists. To be evaluated
before WP-73's fuzz sections are finalised. Note `fuzz_main::<G>()` as a generic
entry point preserves in-process speed while still removing the per-game
boilerplate - consolidation and throughput are not necessarily in conflict.

### RESOLVED 2026-07-26 - `planning/fuzz-throughput-evaluation.md` (Lead-ACCEPTED)

- **Out-of-process is rejected**: 1 request per move, so 1 process spawn per
  move x `num_cpus` threads, plus a second full JSON layer over the state
  payload. Directionally strictly slower. Magnitude UNMEASURED - no cargo in a
  planning session; the commands to settle it are listed in the evaluation.
- **Correction to this decision's premise**: the in-process path is **NOT**
  free of serialisation. `GameRequester` implements the same
  `api::Request`/`api::Response` contract and only drops the transport;
  `Request::Play` carries state as a JSON `String`, so every move already does a
  full state decode + encode, plus a pub render, every player's state JSON and
  N+1 markup renders that the fuzz loop discards. The real gap is a process
  spawn plus a second JSON layer, not "serialised vs not".
- **`fuzz_main::<G>()` confirmed speed-neutral** - same monomorphised
  `fuzz_gamer::<G>` call, only the `main` changes crate. Adopted in WP-73:
  three entry points, 27 `_fuzz` bins kept as 3-line wrappers, `fuzz_gamer`
  kept, only the 27 `_repl` bins deleted.
- **Parallelism is not an available win** - `fuzz()` already runs
  `num_cpus::get()` threads with no shared mutable hot-loop state.
- **OPEN, needs Michael:** evaluation 4(d) is the actual throughput project -
  keep the game live in memory and drive `Gamer` directly, deleting the
  serde/render layer from the hot loop. It trades away the incidental
  render-panic and serialise-panic coverage the current loop gets for free.
  Suggested shape: fast path by default, `--check-renders` for the thorough
  mode, plus one full `renders()` at game end. **Explicitly out of scope for
  WP-73.**

## D-44 - Pivot to SSE now, not after hardening WebSockets

**Ruling: COMMIT to SSE. Migrate NOW, ahead of WP-42's WebSocket hardening.**

Michael's reasoning: the 101-upgrade hijack is what forces WP-42 to hand-roll
pre-upgrade auth. Hardening that machinery and then deleting it is wasted
effort. "I'd like to consider SSE now purely to avoid wasting effort in the
immediate term."

### The framework argument, corrected and strengthened

The evaluation dismissed "modern frameworks only support SSE" on the grounds
that axum 0.8.9 and leptos-use 0.19.0 both ship WS and SSE. That reasoning was
right about the *current* stack but missed Michael's actual motivation, which is
forward-looking and is now on the record:

- The Tokio team (who also maintain axum) have announced a web app framework,
  **Topcoat** - https://github.com/tokio-rs/topcoat. Michael **confirmed with
  them directly in an announcement thread** that Topcoat currently plans to
  support **SSE and not WebSockets**.
- The main Leptos maintainer has recently signalled diminished desire to keep
  developing it. Michael assesses Leptos as carrying a **strong bus-factor
  risk**.
- Leptos has served brdgme well; the strategy is to watch for frameworks from
  significant, established teams as future options.

So the framework argument is not about axum's feature set today. It is about
**not building on a transport that a likely future framework will not support**,
while the maintenance outlook for the current one is uncertain. Treat this as a
real motivation, not a tiebreaker.

Standing note for future agents: do not re-argue this as "axum supports both".
That is true and beside the point.

## D-45 - No `Last-Event-ID` replay

**Ruling: reconnect means "refetch everything visible". Do not emit `id:`.**

NATS Core has no replay, so `Last-Event-ID` would be a promise the server cannot
keep. Michael: "happy with reconnect meaning refetch everything visible so we can
keep implementation as simple as possible. Refetch shouldn't be super expensive."

## D-46 - Connection topology: INVESTIGATE the two-stream option

**Not yet decided.** The evaluation recommended Option C (one stream,
`GET /events?game=<uuid>`). Michael has raised a substantive objection that must
be evaluated before Option C is locked in.

His point: a player may switch between public games within one SPA session.
Under Option C, changing the query param **reopens the single connection**, so
the private/player-data stream drops and reconnects every time the user changes
which public game they are watching - even though nothing about their private
subscription changed.

Proposed alternative: **two SSE connections** -
- a long-lived **private** stream that never drops for navigation, and
- a **public** stream that is swapped as the visible game changes.

Claimed additional benefit: it keeps the door open for further SSE uses, e.g.
**chat**, without disturbing the private stream.

This must be weighed against the ~6-connections-per-origin HTTP/1.1 cap, which
was the evaluation's main argument for a single stream. Note the interaction:
if HTTP/2 is in play on the browser leg the cap largely dissolves; if it is not,
each extra stream is expensive. **Resolve the HTTP/2 question as part of this.**

## D-47 - Cloudflare: Orchestrator's call, with a hard constraint

Michael delegated the rate-limiting decision but set one non-negotiable
requirement: **Cloudflare must not impose a timeout that closes the stream. The
connection must stay open as long as the page is open.**

Ruling: rate-limit **connection establishment** for `/events`, never stream
duration or bytes streamed. Additionally require a **server-side heartbeat**
(an SSE comment line) at an interval comfortably below any proxy idle timeout,
so an idle stream is never reaped. The implementing spec must verify the actual
Cloudflare configuration and any proxy idle timeout rather than assuming.

## D-48 - RESOLVES D-46: browser leg is HTTP/2, so TWO STREAMS

**Measured 2026-07-26 by Michael, not inferred:**

```
$ curl -sI https://brdg.me | head -1
HTTP/2 200
```

The browser leg is HTTP/2 through the Cloudflare edge. This was the single fact
`sse-topology-decision.md` made its recommendation conditional on, so the
conditional resolves to its first branch.

**Ruling: TWO SSE streams.**
- `GET /events` - private, identity-scoped, opened once, **never swapped** on
  navigation.
- `GET /events/public?game=<uuid>` - unauthenticated, swapped as the visible
  public game changes. Needs **no auth and no visibility predicate**, because
  public game ids are already public. This surface reduction, not the reconnect
  cost, is the load-bearing argument.

The ~6-connections-per-origin HTTP/1.1 cap does not bite over h2.

**Hard cap, carried forward: never three held streams.** Future SSE uses (chat,
notifications) must ride the existing private stream. Pending confirmation from
Michael that those uses are private/identity-scoped - a third independently
swapped *public* feed would reopen this decision.

**Dev remains permanently HTTP/1.1** (both Tilt modes plain HTTP, no TLS so no
ALPN, axum has no `http2` feature for h2c). Two streams of the ~6 h1 budget is
comfortable, so this does not change the ruling, but any future stream count
increase must be re-checked against dev, not just production.

WP-84 is unblocked. Finalise it on the two-stream branch and delete the
single-stream fallback shape.

## D-49 - Future SSE uses: keep the door open, build nothing extra

Michael's answer to the "are future uses private?" question, on the record. He
is explicit that these are **hypothetical and may never happen** - none of them
justifies building anything now:

- Private chat messages
- Public chat messages - channels/threads, e.g. game-type specific
- Watching a live tournament (he would like tournaments), possibly including a
  live view of an elimination tree

His read of the likely shape: **one unified private stream carrying multiple
message types**, and **potentially several public channels** (game, tournament,
public chat) - so the "never three streams" cap survives on the private side but
the public side may eventually need more than one topic.

His stated constraint, verbatim: "I don't want to be over-engineering for a
future that may never come, I'd just like us to be aware of potential future use
cases which might help us avoid implementing something in an overly restrictive
way."

**Ruling: build exactly the two streams of D-48. Add no topic machinery, no
multiplexing layer, no channel registry.** The only thing to get right now is to
avoid a shape that would need a redesign later.

### The one cheap generalisation to consider

`GET /events/public?game=<uuid>` bakes "the public stream is about exactly one
game" into the URL. A near-free alternative is a **repeatable topic parameter**,
e.g. `?topic=game:<uuid>`, where today the server accepts exactly one `game:`
topic and rejects everything else. Same behaviour, same code path, same one
connection - but adding `tournament:<id>` or `chat:<channel>` later, singly or
several at once, becomes an additive change rather than a new endpoint or a
change to the connection topology.

WP-84 should **evaluate** this and pick one. If the topic form costs materially
more than the `game` form, take the `game` form - a URL shape is cheap to change
later precisely because nothing persists it. Do not let this grow into a
subscription protocol.

Also worth noting for the eventual design: `event:` field naming on the private
stream is what makes "one stream, multiple message types" work, so keep event
names meaningful from day one rather than sending a single untyped message kind.

### Not decided, deliberately

Whether multiple public topics eventually share one connection or get separate
connections. Revisit only when a second public use case is real. The D-48 hard
cap (never three held streams) stands until then.

## D-50 - Public stream takes a REPEATABLE topic param; N games from day one

Michael asked whether the topic architecture supports watching several public
games at once, and whether to adopt an array form immediately.

**Ruling: `GET /events/public?topic=game:<a>&topic=game:<b>` - the same key
repeated. Parse into a collection from day one. Accept N `game:` topics; reject
every other topic kind.**

### Syntax notes

- Michael's `?topic=game:<a>&game:<b>` is malformed - the second fragment has no
  key. The repeatable form repeats the key.
- **No `[]` suffix.** That is a PHP/Rails convention for explicit array-ness;
  repeated keys already carry it, and `[]` only adds percent-encoding noise.

### Why N now, when D-49 says build nothing extra

These are different axes and only one is speculative:

- Topic **kinds** (`tournament:`, `chat:`) - speculative per D-49. Reject them.
- Topic **count** (several games at once) - a plausible near-term product move
  (lobby/dashboard of live games), and the cost is `Vec<Topic>` instead of a
  scalar plus a fan-out loop.

The trap being avoided: a single-game assumption does not stay in the URL. It
leaks into the subscription bookkeeping and the fan-out path, and that is the
expensive part to undo. Parsing into a collection from day one avoids it even if
the UI only ever passes one topic.

### Required with it

- **Cap N.** An unbounded topic list is a cheap way to make one connection
  expensive.
- **Reject unknown or malformed topics** with an error rather than silently
  ignoring them, so a client bug surfaces immediately instead of appearing as a
  stream that quietly omits things.

### To VERIFY, not assume

axum's `Query` extractor over a `HashMap` collapses duplicate keys, so repeated
params typically need `serde_qs` or a manual parse. Whether that holds at axum
0.8.9 must be checked against the real crate - it was flagged from general
knowledge, not from reading the source.

## D-52 - WP-84 public topic cap = 16

**Ruling: 16.** Michael confirmed the Worker's proposed number rather than
changing it.

Context that shaped it: `/events/public` is **unauthenticated** (D-48 - no
visibility predicate, since public game ids are already public) and is
**deliberately unmatched by any Cloudflare rate rule** (D-47 - navigation
reopens it constantly and the free tier's fixed 10s period trips easily). The
cap is therefore the only bound on what a single connection can ask the server
to watch.

16 sits comfortably above any plausible single-screen game list while keeping
that blast radius small. Note the asymmetry: **raising a cap later is backward
compatible; lowering one breaks existing clients** - so 16 is the safe direction
to be wrong in.

Over the cap is a **400**, not a truncation (per WP-84), so a UI asking for too
many fails visibly rather than silently dropping topics.

Not derived from measurement - nothing was benchmarked this session.

## D-53 - `docs/BACKLOG.md` #54 goes in the "Then" tier, after #31

**Ruling: promote #54 (maximum-performance fuzzer) into the scheduled tier**
alongside #52, #50 and #15 - not the unscheduled post-go-live list.

Michael's reasoning follows the compounding argument: a faster fuzzer makes
every subsequent game port and every remediation package cheaper to validate, so
front-loading it pays back across the rest of the work rather than being
consumed once.

Sequencing note for whoever applies this: #31 (Rust-only repository) lifts
`rust/` to root and reworks the workspace layout - the exact ground WP-73 and the
fuzz bins sit on. #54 sitting *after* #31 is consistent with that.

Reminder on the file's own convention: `NN` is a permanent ID in assignment
order and never implies execution order. Priority lives only in the ordered list
at the top of `docs/BACKLOG.md`, which is what this ruling edits.

## D-51 - Maximum-performance fuzzer: FUTURE WORK, must be persisted

Michael on the 4(d) throughput project surfaced by
`planning/fuzz-throughput-evaluation.md`:

"Those suggestions for a maximum performance fuzzer sound excellent, but I
totally agree it would be future work."

**Ruling: OUT OF SCOPE for WP-73 and for this remediation effort. Do not build
it now. Do persist it properly** so it can be picked up later - Michael asked
explicitly that the discoveries and ideas be written somewhere durable.

### The design he wants recorded, in his own framing

Three fuzzing modes, cheapest to most thorough:

1. **"Game logic only"** (default, maximum speed) - game kept live in memory,
   free of serialisation, no rendering. Drives `Gamer` directly.
2. **Opt-in renders** - pub render **and all private renders after every
   successful command**. Note this is stricter than the current loop, which
   builds renders every move but only via the request contract.
3. **Opt-in serialisation** - the "end to end fuzz", exercising the full
   `api::Request`/`api::Response` path as today.

Michael: "I love the idea of it being totally in memory, being free of
serialisation, and not rendering as the 'game logic only' fuzz, but being able
to opt into renders ... and maybe opting into serialisation as the 'end to end
fuzz' would be cool too."

This supersedes the evaluation's simpler two-mode `--check-renders` sketch: the
axes are **renders** and **serialisation**, and they are independent.

### Why this matters (the case for doing it eventually)

Michael's standing position, from D-43: "the value of the fuzzer is basically
directly correlated by how fast it can run and how many games it can pump
through."

### What must survive into the durable record

- The finding that the in-process path is **not** serialisation-free - every
  move already does a full state decode + encode, a pub render, every player's
  state JSON and N+1 markup renders, of which the loop uses only the acting
  player's `command_spec` and the opaque state string.
- That `fuzz()` is **already parallel** across `num_cpus::get()` threads, so
  parallelism is not an available win.
- The two free wins in `Fuzzer::command`: a whole-`PlayerRender` clone to take
  one field, and a full state-string clone, both per move.
- The tradeoff: the current loop catches render-panics and serialise-panics for
  free; mode 1 gives that up, which is exactly why modes 2 and 3 exist.
- The seven UNKNOWNs and their settling commands in
  `fuzz-throughput-evaluation.md` section 6. **Nothing here has been measured** -
  no cargo in a planning session.

### Home for it

`planning/fuzz-throughput-evaluation.md` already holds the analysis and stays.
Add a `docs/BACKLOG.md` item so it is discoverable outside the review directory,
which is otherwise archival. Cross-reference the evaluation from the item.

## D-42 - `lords-of-vegas-1` gets WP-73 too, but stays undeployed

(Heading restored 2026-07-26 - this ruling was orphaned under D-51's "Home for
it" section with no heading of its own. It is D-42 and has nothing to do with
the fuzzer.)

**Ruling: APPLY WP-73 to `lords-of-vegas-1` like every other game crate.**

It is a workspace member with all four bins but is not deployed - no Dockerfile
stage, no bake target, no Tiltfile entry, no k8s directory. Michael plans to
return to that game and finish it, so it stays a full workspace member and
receives the same treatment.

Note it is **not** a deployment gap to close: leave it undeployed. Only the bin
consolidation applies.

## Terminology correction on the record

Michael referred to this as "the game-bin macro". There is **no macro**. D-20
chose a **generic crate parameterised over the `Gamer` trait plus thin per-game
wrappers**, explicitly rejecting the macro option - and Michael approved it
partly *because* it avoids macros. `brdgme_game_bin` must remain macro-free.
