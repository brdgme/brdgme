# Consolidation notes: units 12-13

Source files: findings/bot-operator-tools.md (unit 12), findings/dependencies.md
(unit 13). Both lead-verified in-session; curated tallies are final. No separate
verification reports exist.

## Unit 12: bot-operator-tools

### Tallies

- bot: 0 critical, 2 major, 9 minor, 5 nit (16)
- operator: 0 critical, 1 major, 4 minor, 3 nit (8)
- tools (fuzz/render_plain/repl): 0 critical, 1 major, 2 minor, 3 nit (6)
- Unit total: 0 critical, 4 major, 15 minor, 11 nit (30)

### Headlines

Criticals: none.

All 4 majors (no IDs assigned; short titles used):

- Reachable unreachable!() in bot retry loop - if the final attempt (19) takes
  a continue path (LLM error or game-state refresh), the for-loop exits and
  hits unreachable!(), panicking the spawned task; message left unacked and
  redelivered. bot/src/main.rs:454 (loop at :242, exhaustion check at :420).
- No ack-deadline extension for long bot turns - turns can run tens of minutes
  (300s LLM timeout x 20 attempts + failover) but the message is acked only
  after completion with no AckKind::Progress heartbeat; JetStream redelivery
  while the original runs yields duplicate command submission. UNCERTAIN on
  consumer ack_wait (owned by monolith). bot/src/main.rs:832-866.
- Hand-rolled finalizer handling in operator - reconciler writes the full
  finalizer array via Patch::Merge from a possibly-stale watch-cache object,
  silently clobbering/resurrecting finalizers edited concurrently by other
  actors; kube::runtime::finalizer::finalizer() exists for this.
  operator/src/controller.rs:80-105.
- Fuzzer hangs forever on worker failure - fuzz() keeps the original step_tx
  Sender alive, so if all workers panic (bad requester path etc.) the channel
  never disconnects and step_rx.recv() blocks forever; user sees panics then a
  silent hang. tools/fuzz/src/lib.rs:23-59.

Notable minors worth surfacing:

- merge_json_patch deviates from claimed RFC 7396 (nulls preserved when target
  entry absent/scalar; literal "budget": null sent to provider).
  bot/src/main.rs:606-631.
- Pervasive try_get(...).unwrap_or(default) masks decode/schema errors
  (mis-typed temperature column silently runs all bots at 0.2; decode error
  can silently disable env-fallback). bot/src/config.rs:35-38 + 7 other sites.
- CRD printcolumn jsonPath .spec.playerCounts targets a nonexistent field -
  kubectl column always empty. operator/src/crd.rs:18.
- GameVersionStatus.message never written; reconcile errors never surface in
  CRD status, only pod logs. operator/src/crd.rs:43, controller.rs:155-160,223-226.
- No graceful shutdown in bot - SIGTERM kills tasks mid-LLM-call (wasted spend,
  stalled game until redelivery); tokio "signal" feature enabled but unused.
  bot/src/main.rs:812-869.

### Unit state

Scope is bot/ (1,708 LOC), operator/ (412 LOC), tools/fuzz (358) plus two tiny
tools, ~2,662 LOC total; zero criticals and a long clean list (crypto, NATS
event structs, routing failover, template rendering, operator upserts, both
small tools fully clean). Dominant problem classes: lifecycle/robustness gaps
in long-running async work (panic-on-edge-case loop shape, no ack heartbeat,
no graceful shutdown, unbounded spawn) and silent error-swallowing in config
loading. Operator's issues are mostly framework-idiom departures (hand-rolled
finalizers, ad-hoc json! status patches) rather than logic bugs.

### Theme evidence

- Request-reachable panics: reachable unreachable!() in the bot turn loop
  (main.rs:454); fuzzer SystemTime duration_since panic on clock step
  (fuzz/lib.rs:46-53); fuzzer exit-signal send unwraps (lib.rs:87-89). The
  bot panic is event-reachable (NATS message), not HTTP-request-reachable.
- TOCTOU/concurrency: ack-after-all-work with no Progress heartbeat ->
  duplicate turn processing (explicitly cross-referenced to the same pattern
  in web-server/web-domain units); operator finalizer merge-patch race on
  stale watch-cache data; unbounded tokio::spawn per message.
- Error-swallowing: try_get().unwrap_or(default) across config.rs/main.rs;
  LLM_EXTRA_BODY parse failures silently yield None (a test enshrines it);
  load_key silently returning the insecure default key; dropped JoinHandle
  hiding task panics.
- Unmaintained/deprecated/duplicated deps: serde_yaml 0.9 archived (bot);
  num_cpus duplicating std available_parallelism (fuzz); direct getrandom for
  a nonce aes-gcm can generate itself (also the 0.3-vs-0.4 drift crate);
  unused deps time + brdgme_markup and unused tokio signal feature in bot;
  k8s-openapi "latest" feature instead of pinned cluster version.
- Boilerplate/duplication: hand-rolled {{player N}} substitution instead of
  the workspace brdgme_markup library (with a name-injection quirk);
  minijinja Environment rebuilt per render.
- Version drift: getrandom (cross-ref to unit 13).

### Discussion candidates

- Ack heartbeat vs ack_wait sizing for bot turns: consumer config is owned by
  the monolith and invisible to this crate - needs a cross-crate decision on
  Progress heartbeats vs raising ack_wait, plus bounding worst-case turn time.
- Operator finalizer migration to kube-rs helper: restructures reconcile into
  apply/cleanup closures - mechanical-ish but changes the reconciler's shape;
  bundle with typed-status and error_policy status-writing decisions.
- Graceful shutdown for the bot: needs a policy on termination grace period
  and whether to await in-flight multi-minute LLM calls.
- serde_yaml replacement: must be coordinated with lib/game_client (unit 13) -
  choice of fork vs JSON output is a design call.

## Unit 13: dependencies

### Tallies

- Unit total: 0 critical, 4 major, 17 minor, 5 nit (26)
- (Raw: W1 structure 2M/8m/2n, W2 currency 2M/9m/4n; one chrono duplicate
  merged in curation.)

### Headlines

Criticals: none.

All 4 majors:

- No [workspace.dependencies] - shared versions copy-pasted across 40
  manifests; serde in 36 crates, rand 33x, tokio 33x, mixed precise-vs-major
  spellings already drifting. Root Cargo.toml:1.
- sqlx split 0.8 (web) vs 0.9 (bot, operator) - both stacks in Cargo.lock,
  double compile plus two type-mapping behaviours against one database; web
  likely pinned by tower-sessions-sqlx-store 0.15. web/Cargo.toml:28,
  bot/Cargo.toml:16, operator/Cargo.toml:28.
- sentry default features drag actix-web 4 (8 actix-* packages) and ureq 3
  (third HTTP client, with native-tls/openssl) into every server build - all
  four sentry declarations on default features; nothing uses actix or ureq.
  bot/Cargo.toml:21, web/Cargo.toml:86, lib/cmd/Cargo.toml:19,
  lib/game_client/Cargo.toml:16.
- term_size 0.3.2 unmaintained (RUSTSEC-2020-0163, archived since 2020,
  currently ignored in deny.toml) - direct dep of brdgme_cmd, linked by every
  game binary; maintained successor terminal_size has the same API.
  lib/cmd/Cargo.toml:16.

Notable minors:

- 27 game crates x 4 boilerplate binaries = 108 near-identical files (~38
  lines/crate varying only in the crate name), also forcing brdgme_cmd/
  brdgme_fuzz/tokio deps into every game manifest. game/*/src/bin/.
- deny.toml bans multiple-versions = "warn" with empty skip lists; known
  duplicates (3x rand, 3x getrandom, 2x sqlx) never fail CI; sources checks
  and wildcards also warn/allow. deny.toml:69-70,80-81.
- 4 of 7 advisory ignores stale (diesel/encoding crates absent from lock).
  deny.toml:19-27.
- Game-crate tokio features = ["full"] 27x for a 13-line http bin; general
  feature-set drift on shared deps. game/*/Cargo.toml.
- getrandom 0.3/0.4 split (bot vs web); lock carries 0.2/0.3/0.4.
- web pins tower-http/gloo-net/gloo-timers one step ahead of leptos/reqwest,
  doubling each crate (gloo-net duplication plausibly costs WASM bytes).
- serde_yaml 0.9.34+deprecated with two direct consumers (bot + game_client);
  fixing only bot leaves it in the tree.
- lib/cost has one consumer (seven-wonders-1) while splendor-2 reimplements
  the same cost bookkeeping locally in 155 lines.

### Unit state

Scope is the root manifest, all 40 member Cargo.tomls, Cargo.lock (709
packages, queried mechanically), deny.toml, rust-toolchain.toml. Core stack
currency is genuinely good (tokio/serde/axum/leptos/aes-gcm etc. current;
known-bad crates all absent from the lock); the dominant problem class is
absent workspace-level unification ([workspace.dependencies], [workspace.package],
[workspace.lints]) which has already produced version drift, plus oversized or
split dependency choices (sentry defaults, sqlx 0.8/0.9, warp-vs-axum,
chrono-vs-time). deny.toml exists but is toothless at warn level with stale
ignores.

### Theme evidence

- Missing [workspace.dependencies] unification: the headline structural major;
  also no [workspace.package] (authors already drifted: missing from bot/web/
  operator) and no [workspace.lints]; duplicated [profile.wasm-release] in
  web/Cargo.toml silently ignored by cargo (root copy adds debug = true -
  live divergence).
- Version drift: sqlx 0.8/0.9 (major); getrandom 0.2/0.3/0.4; rand 0.8/0.9/
  0.10 (3 full stacks, largest duplicate cluster after actix); tower-http/
  gloo-net/gloo-timers one-ahead pins; chrono-vs-time (rand_bot only chrono
  user); convert_case x3; svix holding http 0.2 in the tree.
- Security advisories (RUSTSEC): term_size RUSTSEC-2020-0163 (major, direct,
  actionable); paste RUSTSEC-2024-0436 (transitive via leptos, ack-only);
  4 stale deny.toml ignores for advisories whose crates left the lock.
- Oversized feature sets: sentry defaults -> actix + ureq (major); tokio
  "full" in all 27 game crates; k8s-openapi "latest" (unit 12).
- Unmaintained/deprecated/duplicated deps: term_size, serde_yaml (+archived
  unsafe-libyaml backend), combine 4.6 dormant at the heart of the markup
  parser, warp single-maintainer beside axum, env_logger/log split vs tracing,
  num_cpus and lazy_static superseded by std.
- Boilerplate duplication (108 game binaries): 27 crates x 4 bins; lib/cost
  half-shared (one consumer + one local reimplementation in splendor-2).
- Policy gap: deny.toml multiple-versions/sources/wildcards at warn/allow;
  no mechanical currency check (recommendation: cargo outdated or scheduled
  cargo-deny in CI).

### Discussion candidates

Confirmed expected candidates:

- sqlx 0.8/0.9 unification path: web is blocked on tower-sessions-sqlx-store;
  choose between waiting for/bumping to an sqlx-0.9-compatible release or
  vendoring the trivial session-store impl, then move sqlx to workspace deps.
- sentry feature trim: which feature set to keep (backtrace/contexts/panic/
  debug-images/reqwest/native-tls suggested); needs cargo tree verification
  that actix/ureq drop out; native-tls transport is documented as deliberate
  so the trim must preserve that choice.
- [workspace.dependencies] migration: 40-manifest touch; sequencing with
  [workspace.package], [workspace.lints], and per-crate feature additions;
  natural umbrella for several other fixes.
- 108 boilerplate game binaries: macro (brdgme_game_bins!(Game)) vs one
  generic parameterised bin crate; decision affects every game manifest and
  where tokio/fuzz deps live.

Additional candidates:

- serde_yaml migration: two consumers (bot + game_client) must move together;
  fork (serde_yaml_ng/serde-yml/saphyr - kube already pulls serde-saphyr) vs
  switching the surface to JSON.
- warp -> axum consolidation in lib/cmd: small surface but touches all 28
  game binaries' HTTP layer.
- deny.toml hardening: flipping multiple-versions to deny requires
  enumerating/accepting current duplicates in skip/skip-tree first.
- combine dependency: accept as recorded risk vs migrate brdgme_markup to
  winnow/in-house combinator when next touched.
- lib/cost: fold into seven-wonders-1 vs port splendor-2 onto it; status quo
  (half-shared) called the worst option.

## Cross-unit observations

- getrandom appears in both units: unit 12 recommends dropping bot's direct
  dep (use aes-gcm generate_nonce), which also resolves part of unit 13's
  0.3/0.4 drift finding.
- serde_yaml flagged in both; unit 13 establishes the fix must cover
  lib/game_client too, not just bot.
- num_cpus flagged in both (fuzz); same std replacement.
- Unit 12's ack-after-all-work finding explicitly cross-references the
  identical pattern in the web-server/web-domain units - a workspace-wide
  NATS-consumer theme (no Progress handling, config owned by monolith).
