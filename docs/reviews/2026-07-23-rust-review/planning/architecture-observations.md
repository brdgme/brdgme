# Architecture observations - running notes for the deferred architectural review

Started 2026-07-26 by the WP-82 (`db.rs` module split) spec Lead.

**What this file is.** The broader architectural review - oversized functions,
oversized types, oversized files, crate splitting, module-tree flattening - is
**deferred to a separate follow-up session after remediation**. No remediation
package may widen into it. This file is where structural observations are
parked in the meantime so they are not lost.

**Rules for appending.**

- Brief bullets. File path plus symbol name. No analysis, no proposed fix.
- Identify code by **function/type name**, never by line range.
- Append; do not rewrite other Leads' entries.
- One `##` section per contributing unit, newest last.

---

## From the WP-82 `db.rs` module split unit (2026-07-26)

Observed while inventorying `rust/web/src/db.rs` and its 293 external call
sites. None of this is in WP-82's scope - WP-82 is a pure move.

**Oversized items**

- `web/src/db.rs` - the `#[cfg(all(test, feature = "ssr"))] mod tests` is
  **4838 lines, 59% of the 8149-line file**. Even after WP-82 splits the
  production code, tests dominate. Candidates for `web/tests/` integration
  tests: everything binding only to `pub` API (all but `choose_colors`,
  `elo_rating_change`, `apply_rating_changes`).
- `db.rs::create_game_with_users_tx` (~205 lines) - username generation,
  colour assignment, player insert, bot insert, game insert in one body.
- `db.rs::apply_rating_changes` (~151 lines) - pure pairwise ELO maths mixed
  with the SQL read/write loop. Splits into gather / compute / persist.
- `db.rs::update_game_command_success` (~115 lines) - hottest write path,
  enough arguments to need `#[allow(clippy::too_many_arguments)]`. A
  parameter struct is indicated.
- `db.rs::find_game_extended` (~113 lines).

**Duplication**

- `db.rs::friend_recent_visible_game` inlines the
  `db.rs::is_game_visible_to_user` predicate as SQL, guarded only by a test
  asserting the two agree.
- Four pool-vs-connection duplicate pairs exist purely for the executor
  split: `has_block`/`has_block_conn`,
  `find_open_restart_proposal`/`find_open_restart_proposal_tx`,
  `create_game_with_users`/`create_game_with_users_tx`,
  `create_game_logs`/`insert_game_logs_tx`. A generic `impl Executor` bound
  would collapse all four.

**Misplaced responsibility / inverted dependencies**

- `db.rs` imports `crate::game::server_fns::BotSlot` and
  `crate::game::StatusUpdate` - the data layer depending on the server-fn
  layer. Both look like `models/` types.
- `db.rs::cap_digest<T>` is generic `Vec` truncation with no database
  involvement.
- `db.rs::validate_username` is the single ungated item in an otherwise
  server-only module, purely so the client form shares it. A shared-validation
  concern, arguably `models/` or a small shared crate.
- `db.rs::normalize_pref_color` (`pub(crate)`) is consumed by
  `stats/queries.rs` and `email/commands.rs` - presentation reaching into the
  DB layer for a string helper.

**Boundaries**

- No transaction abstraction: every write calls `pool.begin()` inline and
  hand-rolls commit/rollback.
- No repository/port boundary: 293 external `db::` references across 22 files
  all bind to concrete functions, so the DB layer is not substitutable.
- Row-mapping is inconsistent: `build_game_type_user` and
  `build_game_player_from_row` take positional args (both need
  `#[allow(clippy::too_many_arguments)]`) while `PendingGameRow`,
  `FinishedGameRow` and `FriendRow` use `#[derive(sqlx::FromRow)]`.
- `db.rs::is_user_recently_active` returns `bool`, not `Result<bool>` - it
  swallows DB errors, unlike every other query fn.

**From the dependency cluster (WP-66/67/69 spec pass, 2026-07-26)**

- `rust/bot/src/main.rs` is ~920+ lines and holds `main`, sentry init, tracing
  init, and the NATS request handler with `start_transaction` in one file.
- `rust/web/src/auth/session.rs` mixes the wire type `SessionUser`
  (non-gated) with the entire ssr-only store/layer construction
  (`create_session_layer`, `set_user_session`, `get_user_from_session`) behind
  per-item `#[cfg(feature = "ssr")]` rather than a module split.
- `rust/lib/cmd/src/http.rs::serve` couples env_logger init, sentry init, warp
  route construction and SIGTERM shutdown in one function.
- Sentry initialisation is copy-pasted in three places with identical
  `ClientOptions` (`web/src/main.rs::init_sentry`, `bot/src/main.rs::main`,
  `lib/cmd/src/http.rs::serve`) - candidate for one helper in a shared crate.

- `rust/Cargo.toml` `members` array is hand-maintained at 40 entries and
  mostly-but-not-quite alphabetical; a `game/*` glob would remove the per-game
  root edit entirely.
- All 27 `rust/game/*/Cargo.toml` are byte-near-identical (same five
  `brdgme_*` path deps + rand/serde/tokio); candidate for a shared bin crate
  rather than 27 manifests.
- `rust/web/Cargo.toml` is ~190 lines with a 54-entry `ssr` feature list -
  the largest manifest in the tree by a wide margin.
- `rust/lib/game_client/src/lib.rs` is ~738 lines holding retry policy
  (`backoff_delay`, `send_with_retry`), the request/response mapping, the
  `GameData` aggregate, `json_to_yaml`, and the tests in one module.
- `rust/lib/cmd/src/http.rs` carries `env_logger::init()` inside
  `serve`, while `env_logger` is an ungated dependency of `brdgme_cmd`.

## From the WP-73 game-binary-consolidation unit (2026-07-26)

Observed while inventorying the 108 per-game bin targets. Out of WP-73's scope.

- **`rust/tools/fuzz/src/main.rs` and `rust/tools/repl` are ALREADY generic,
  out-of-process equivalents** of the per-game `_fuzz`/`_repl` bins - they drive
  a game through `requester::parse_args` / `LocalRequester`, which shells out to
  a `_cli` binary. So the workspace carries two parallel fuzz paths and two
  parallel repl paths (in-process generic vs out-of-process generic). Whether
  both are still wanted is an architectural question, not a remediation one.
- **`lords-of-vegas-1` is a workspace member with all four bins but is not
  deployed**: no `rust/Dockerfile` stage, no `docker-bake.hcl` target, no
  `Tiltfile` entry, no `k8s/base/game/lords-of-vegas-1/`. 27 crates, 26 deployed.
  Its status (unfinished? retired?) needs a human decision.
- **`k8s/base/game/` holds 44 directories** but only 26 correspond to a live
  Rust crate - the remainder are retired game versions. Drift between the k8s
  tree and the workspace.
- **No in-repo crate depends on any game crate as a library** (zero `path =
  "../../game/..."` deps). Game crates are leaves reached only over HTTP.
- `rust/lib/cmd` mixes three unrelated front-ends (`cli`, `repl`, `http`) plus a
  `requester` module and `test_support` in one crate, with `http-server`
  default-on; that default is why every game crate transitively pulls
  warp+tokio+sentry.

## From the Batch 6 spec unit - WP-81 / WP-17 / WP-83 (2026-07-26)

Observed read-only while confirming three specs. None of it is in scope for
those packages.

**Duplicated implementations**

- `rust/game/splendor-2/src/cost.rs` is a hand-ported subset of
  `rust/lib/cost/src/lib.rs` (`from_resources`/`add`/`inv`/`sub`/`sum`/
  `can_afford`), verified semantically equivalent. WP-17 removes this one, but
  the pattern - a game crate re-porting a `brdgme-go` lib rather than depending
  on the Rust one - is worth checking for elsewhere. `lib/cost` had exactly ONE
  consumer (`seven-wonders-1`) before WP-17.

**Dead machinery**

- Per-game `Stats` structs are written but not consumed by any platform path.
  `acquire-1`'s whole `src/stats.rs` had zero callers; `lost-cities-1/-2`
  carried a never-touched `investments` field and an `expeditions` counter that
  counted something other than its name. WP-81 deletes these three, but the
  underlying gap - **there is no platform stats consumption path at all**, so
  `Gamer::player_stats` output goes nowhere obvious - is a feature-shaped
  question Michael has flagged for a clean-slate revisit, not a remediation one.

**Determinism hazards**

- Game setup paths shuffle with a seeded RNG, so any grouping/collection step
  introduced into setup must use an ordered container. `seven-wonders-1`'s
  `start_game` is the live example (WP-83 section 2 mandates `BTreeMap`). Worth
  a general convention check: `HashMap`/`HashSet` iteration anywhere upstream of
  a seeded shuffle silently breaks game reproducibility.

**Data modelled in strings**

- `seven-wonders-1`'s `card::cities()` encodes the A/B side of a physical wonder
  board **only in the `name: String` suffix** (`"Rhodes A"` / `"Rhodes B"`).
  There is no `Side` enum and no board identifier, so the only way to know two
  entries are the same physical board is string-suffix stripping. WP-83 does
  exactly that as the minimal fix; a typed representation is the structural
  answer.

**Out-of-process game boundary already exists and is underused**

- `rust/lib/cmd/src/requester/local.rs`'s `LocalRequester` drives a game purely
  as a child process over stdin/stdout JSON, given a binary path at runtime;
  `rust/tools/fuzz` and `rust/tools/repl` are three-line generic drivers on top
  of it. The 27 per-game `_fuzz`/`_repl` bins were in-process duplicates of
  capability that already existed generically (WP-73 / D-41 deletes them).
  Michael's stated rationale is that this out-of-process boundary is what would
  make **non-Rust game implementations** viable again - so the `_cli` binary
  contract (one JSON `Request` in, one JSON `Response` out) is the real
  cross-language interface, not the `Gamer` trait. Worth treating `_cli`'s
  filename and protocol as a frozen public API on that basis, not just because
  the Dockerfile happens to copy `_http`.
- Cost of the boundary, measured only by reading: `LocalRequester::request`
  spawns a fresh process per API call, so anything driving many requests
  (fuzzing especially) pays a process spawn each time. If out-of-process
  fuzzing becomes the only fuzzing, a persistent-child or batched protocol is
  the obvious next step.

## WebSocket transport (from the WS->SSE evaluation, 2026-07-26)

- The WebSocket at `/ws` uses **none** of WebSocket's defining capability. There
  is zero client->server application traffic: `handle_socket`'s inbound arm binds
  the frame to `_` and discards it, and the client drops leptos-use's `send`
  handle. The reverse channel carries only browser auto-pongs and close frames.
- The socket carries only cache-invalidation pings (`{"game_id":..}` /
  `{"proposal_id":..}`), never content. All authorization happens on the
  subsequent server-fn refetch. This is a good property and should be preserved
  under any transport.
- Every connection subscribes to the NATS wildcards `game.>` and `proposal.>`
  with **no server-side filtering** - every client sees every signal
  system-wide. `docs/ARCHITECTURE.md` overstates this as targeted fan-out.
- The upgrade (101) detaches the socket from axum's request tracking, which is
  the sole reason WP-42 must hand-roll pre-upgrade auth and `GameBroadcaster`
  must carry a `TaskTracker` for drain. An ordinary streaming HTTP response
  would need neither.
- `k8s/base/ingress/` and `k8s/base/ingress-nginx/` are **dead config** -
  referenced by no kustomization. The nginx `proxy-read-timeout: 604800` there
  does not apply to the live Cilium/Envoy Gateway deployment. Candidate for
  deletion in the architectural review.
- The repo nowhere expresses an HTTP protocol version. The origin is built
  HTTP/1.1-only (axum's `http2` feature is not enabled anywhere); the browser
  leg is not determinable from config. Any design assuming HTTP/2 multiplexing
  is unverified, and dev (`http://web.brdgme.lvh.me:8080`, no TLS) is almost
  certainly HTTP/1.1.
  **ANSWERED 2026-07-26 - the config claim stands, the open question does not.**
  The repo still expresses no protocol version, but Michael measured the browser
  leg directly: `curl -sI https://brdg.me | head -1` -> `HTTP/2 200`. See D-48.
  The observation that survives for the architectural review is the *original*
  one: a production-relevant protocol fact is discoverable only by hitting the
  live edge, because the Cloudflare zone's protocol settings live in the
  dashboard and not in `infra/cloudflare.tf`, which sets only `ssl` and
  `websockets`. Dev is confirmed permanently HTTP/1.1.
- Full evaluation: `planning/ws-to-sse-evaluation.md`; inventory:
  `planning/raw/websocket-inventory.md`.

## SSE topology investigation (D-46), 2026-07-26

- **No HTTP protocol version is expressed anywhere in the repo.** `infra/cloudflare.tf`
  configures exactly two zone settings, `ssl = "strict"` and `websockets = "on"`.
  There is no `http2`/`http3`/`zero_rtt`/`min_tls_version` setting and no
  `cloudflare_zone_settings_override` resource. Grepping `k8s/`, `infra/`,
  `Tiltfile`, `docker-bake.hcl` and `rust/Dockerfile` for
  `http2|http/2|h2c|alpn|http3|appProtocol` returns zero hits. A load-bearing
  operational property of the system is therefore unknowable from the repo. If
  it matters (and for connection-topology decisions it does), it should be
  asserted in Terraform rather than left to a dashboard default.
- **Dev is permanently HTTP/1.1.** Both Tilt modes are plain HTTP: default runs
  `cargo leptos watch` on `http://localhost:3000`; `WEB_IN_CLUSTER=1` uses a
  `brdgme-dev` Gateway whose only listener is `port: 80, protocol: HTTP`. No TLS
  means no ALPN, and axum has no `http2` feature so h2c-with-prior-knowledge is
  unavailable. Any concurrency design must be sized for the ~6-connections-per-
  origin cap in dev even if prod is h2 - this is a class of bug that never
  reproduces in prod.
- **`k8s/base/gateway/httproutes.yaml` configures no session affinity** and the
  web deployment runs `replicas: 2`. Any design placing a stateful long-lived
  connection on one replica and a control-plane request on another is broken by
  default. This killed the "side-channel POST resubscribe" SSE shape and is a
  general constraint on future stateful-connection work.
- **The client's `(id, seq)` trigger design is idempotent under duplicate
  delivery** (`bump_game_update` / `track_game_seq`), which makes multi-stream
  and firehose transports safe without server-side dedup. Worth preserving.

## Fuzz / requester layering (Lead + Worker, 2026-07-26, D-43 evaluation)

Source: `planning/fuzz-throughput-evaluation.md`. Lead independently verified the
first three points against `rust/tools/fuzz/src/lib.rs`,
`rust/lib/cmd/src/requester/gamer.rs` and `rust/lib/cmd/src/requester/local.rs`.

- **The `Requester` abstraction forces a JSON boundary even when there is no
  transport.** `api::Request::Play` carries game state as `game: String`
  containing JSON, so the "in-process" `GameRequester` still does a full
  `serde_json::from_str::<G>` and `serde_json::to_string` of game state on every
  single request. The abstraction was built for the out-of-process case and the
  in-process implementation conforms to it rather than short-circuiting it. Any
  future performance work on this path has to break that symmetry.
- **`renders()` is unconditional and eager.** Every `New`/`Play`/`Status`
  response builds the public render plus every player's state JSON plus every
  player's markup render, regardless of what the caller wants. The fuzzer uses
  one `command_spec` out of all of it. The web path presumably wants all of it,
  so the cost is structural, not accidental - but there is no way to ask for
  less. A `Request` variant or flag expressing "what do you actually need" would
  serve both callers.
- **The fuzz loop is already saturating all cores** (`num_cpus::get()` threads,
  no shared mutable hot-loop state). Throughput headroom is entirely per-move
  work, not concurrency.
- **`LocalRequester` spawns a process per request, not per session.** There is
  no persistent-child mode and `cli.rs` is a one-shot by construction (reads one
  `Request` from stdin, writes one `Response`, returns). If out-of-process game
  implementations are ever revived for real workloads, this is the first thing
  that has to change; a newline-delimited persistent protocol would be the
  obvious shape.
- **No `Rc`/`RefCell`/`Cell` anywhere under `rust/game/*/src/`.** Game state is
  plain data throughout, so games are `Send`-friendly by construction even
  though the `Gamer` trait does not require it.

## Query-string parsing - no server-side pattern exists at all

Observed 2026-07-26 while finalising WP-84 (SSE). Verified by grep over `rust/`:
`extract::Query`, `RawQuery` and `axum_extra` have **zero hits tree-wide**.
`rust/web/src` has never parsed a query string on the server side - every route
is either a fixed path, a Leptos server-fn POST, or `/ws`. All query handling in
the codebase is client-side, through leptos_router's `use_query_map()`
(`rust/web/src/new_game.rs`, `rust/web/src/players.rs`).

WP-84's `/events/public?topic=game:<id>` will be the **first server-side query
parameter in the web crate**. Two consequences worth carrying into the deferred
architectural review:

- There is no house convention to follow, so whatever WP-84 does becomes the
  convention by default. It is worth being deliberate about it once rather than
  letting the second and third query-taking route each invent their own shape.
- A non-obvious axum trap sits right at that boundary: `serde_urlencoded` (what
  `axum::extract::Query` delegates to) deserializes each value through a `Part`
  deserializer that forwards `seq` to `deserialize_any`, so **the intuitive
  `struct Params { topic: Vec<String> }` form 400s** and
  `HashMap<String, String>` silently keeps only the last duplicate.
  `Query<Vec<(String, String)>>` is the form that actually preserves repeated
  keys. Anyone adding a second repeatable parameter later will hit this again.

Related: `serde_qs` is already in `rust/Cargo.lock` at 0.15.0, but only
transitively via `leptos`/`server_fn`, while crates.io latest is 1.1.2. Adding it
as a direct dependency at latest would put two majors of it in the tree - a
concrete case where the standing "dependencies at latest" preference collides
with a transitive pin owned by the framework. Worth a look during the dependency
sweep rather than at the point of need.
