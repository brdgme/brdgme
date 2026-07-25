# Raw findings: dependency currency (Unit 13 W2)

Scope: version currency, lockfile duplication, and maintenance health of dependencies across the 41 manifests at rust/ (snapshot f8763a5, Cargo.lock 709 packages). Manifest structure, [workspace.dependencies], sqlx 0.8/0.9 and getrandom 0.3/0.4 drift, deny.toml, and game-binary boilerplate are covered by the parallel structure worker and are not re-litigated here; where a duplicate stack in the lock is purely a consequence of that drift it is cross-referenced, not re-flagged.

### sentry default features drag actix-web 4 and ureq 3 into every server build
- severity: major
- category: dependencies
- location: bot/Cargo.toml:21 (also web/Cargo.toml:86, lib/cmd/Cargo.toml:19, lib/game_client/Cargo.toml:16; Cargo.lock:5377)
- finding: all four sentry declarations are `sentry = "0.48"` with default features. The locked `sentry 0.48.5` package depends on `sentry-actix`, which pulls the full actix-web framework (`actix-web 4.14.0`, `actix-http`, `actix-router`, `actix-rt`, `actix-server` - 8 `actix-*` packages in the lock), plus `ureq 3.3.0` (a third HTTP client alongside reqwest and hyper) with `native-tls`/`openssl`. The workspace serves HTTP with axum and warp only; actix is dead weight compiled into bot, web (ssr), operator (via game_client), and all 28 game binaries (via brdgme_cmd's http-server feature). The native-tls/openssl transport itself is documented as deliberate (web/Cargo.toml comment), but nothing uses actix or ureq.
- recommendation: declare sentry with `default-features = false` and only the needed features (e.g. `backtrace`, `contexts`, `panic`, `debug-images`, `reqwest`, `native-tls`), ideally once via `[workspace.dependencies]`. Verify with `cargo tree -i actix-web` that the actix and ureq subtrees drop out of the lock.

### term_size 0.3.2 is unmaintained (RUSTSEC-2020-0163)
- severity: major
- category: dependencies
- location: lib/cmd/Cargo.toml:16 (Cargo.lock:6549)
- finding: `term_size = "0.3.2"` is a direct dependency of brdgme_cmd, which every game binary and the repl link. term_size has been archived/unmaintained since 2020 and carries the RUSTSEC-2020-0163 unmaintained advisory; last release 0.3.2 (2018). This is a direct violation of the "well-maintained, battle-hardened" charter and will trip `cargo deny check advisories` unless it is being ignored in deny.toml.
- recommendation: replace with `terminal_size` (the maintained successor, same one-call API) in lib/cmd.

### combine 4.6 parser library is dormant in the two core parsing crates
- severity: minor
- category: dependencies
- location: lib/game/Cargo.toml:12 and lib/markup/Cargo.toml:10 (Cargo.lock:1076)
- finding: `combine = "4.6.7"` backs the brdgme_markup parser and parts of brdgme_game. combine's development has been effectively dormant for years (4.6.x is the terminal series; the announced 5.0 rewrite never shipped), and the ecosystem has consolidated on nom/winnow for actively maintained parser combinators. Not an advisory, but a low-momentum dependency at the heart of the markup pipeline. The custom parser combinator in lib/game is deliberate and not flagged; this is specifically about the third-party combine dependency sitting next to it.
- recommendation: no urgent action; when brdgme_markup is next touched substantially, consider migrating to winnow (or folding into the existing in-house combinator). Record as accepted risk otherwise.

### serde_yaml is deprecated/archived - workspace-level view
- severity: minor
- category: dependencies
- location: lib/game_client/Cargo.toml:15 (also bot/Cargo.toml:29; Cargo.lock resolves `serde_yaml 0.9.34+deprecated`)
- finding: the lock literally records `0.9.34+deprecated`; dtolnay archived serde_yaml in 2024 (RUSTSEC-2024-0320-adjacent unmaintained status), and its backend `unsafe-libyaml 0.2.11` is likewise archived. A Unit 12 finding already covers the bot usage; workspace-wide there are exactly two direct consumers: bot and lib/game_client. Fixing only bot would leave the archived crate in the tree via game_client.
- recommendation: migrate both consumers together to a maintained YAML crate (e.g. serde_yml successor or the saphyr family - note kube already pulls `serde-saphyr` transitively) or to JSON/TOML if the YAML surface is internal-only.

### paste 1.0.15 (unmaintained, RUSTSEC-2024-0436) via the leptos stack
- severity: minor
- category: dependencies
- location: Cargo.lock (paste 1.0.15; dependents: leptos 0.8.20, leptos-use 0.19.0, reactive_graph, reactive_stores, tachys, either_of) pulled by web/Cargo.toml:19
- finding: `paste 1.0.15` carries the RUSTSEC-2024-0436 unmaintained advisory. It is purely transitive from the leptos 0.8 ecosystem (six dependents, all leptos-family), so it is not actionable in this repo beyond acknowledging it - proc-macro-only, build-time, no runtime exposure. Notably atty, proc-macro-error, dotenv, instant, and ansi_term (the other usual unmaintained suspects) are all absent from the lock.
- recommendation: ignore RUSTSEC-2024-0436 explicitly in deny.toml with a comment naming leptos as the source, so the advisory ignore list stays auditable; revisit when leptos drops paste.

### three parallel rand stacks in the lock (0.8 / 0.9 / 0.10)
- severity: minor
- category: dependencies
- location: Cargo.lock (rand 0.8.7, 0.9.5, 0.10.2; rand_core 0.6.4/0.9.5/0.10.1; rand_chacha x3; getrandom 0.2.17/0.3.4/0.4.3)
- finding: the workspace is uniformly on rand 0.10.2 (all 40 first-party consumers), but three full rand stacks compile anyway: rand 0.9.5 via leptos, governor (axum-prometheus), sentry-core/types, metrics-util, tungstenite 0.29; rand 0.8.7 via nkeys/nuid (async-nats), num-bigint-dig, sqlx 0.8's mysql/postgres drivers, tokio-websockets, tower-sessions-core. getrandom 0.2.17 rides in via ring and nkeys. First-party code is clean; this is ecosystem-era lag among direct deps' own trees plus the sqlx 0.8/0.9 split (structure worker's finding). Mostly unavoidable today, but it is the largest build-bloat duplicate cluster after actix.
- recommendation: no first-party change possible for most; the sqlx-driven 0.8 copies disappear when the sqlx 0.8/0.9 split is resolved (cross-ref structure worker). Re-check after the next leptos/async-nats upgrades whether 0.9/0.8 stacks drop out.

### web pins tower-http 0.7 / gloo-net 0.7 / gloo-timers 0.4 one step ahead of their ecosystems, doubling each crate
- severity: minor
- category: dependencies
- location: web/Cargo.toml:41,75,76 (Cargo.lock: tower-http 0.6.11+0.7.0, gloo-net 0.6.0+0.7.0, gloo-timers 0.3.0+0.4.0)
- finding: web declares tower-http 0.7, but axum-prometheus, kube-client, leptos_axum, and reqwest all still consume tower-http 0.6.11, so both versions build. Same pattern for gloo-net (leptos_router/server_fn on 0.6, web on 0.7 - two copies in the WASM bundle, where size matters given the wasm-release opt-level = "z" effort) and gloo-timers (backon on 0.3). Being one minor ahead of leptos buys nothing and costs a duplicate compile of each; for gloo-net it plausibly costs WASM bytes.
- recommendation: pin web to the versions its frameworks use (tower-http 0.6, gloo-net 0.6, gloo-timers 0.3) until leptos/reqwest move, or verify the newer APIs are actually required.

### svix 1.98 pulls both http 0.2 and http 1.x
- severity: minor
- category: dependencies
- location: Cargo.lock:6429 (svix; pulled by web/Cargo.toml:50)
- finding: svix's lock entry depends on `http 0.2.12` and `http 1.4.2` simultaneously; svix and the actix stack (see sentry finding) are the only things keeping the legacy http 0.2 line in the tree. svix also duplicates itertools (0.15.0 vs the leptos/mrml ecosystem's 0.14.0). Vendor SDK, so limited leverage, but it is the marker of the one remaining pre-hyper-1.0 era corner of the lock.
- recommendation: check whether a newer svix release drops the http 0.2 dependency next time deps are refreshed; if the sentry/actix fix lands, svix will be the sole http 0.2 holdout.

### chrono in rand_bot vs time everywhere else
- severity: minor
- category: dependencies
- location: lib/rand_bot/Cargo.toml:11
- finding: brdgme_rand_bot is the only crate in the workspace using chrono (0.4.45, serde feature); every other crate (cmd, game, bot, web) standardizes on time 0.3. Two full date/time libraries compile for one consumer, and the workspace loses type-compatibility between rand_bot timestamps and everything else.
- recommendation: port rand_bot's chrono usage to time 0.3 and drop chrono from the lock.

### env_logger/log in brdgme_cmd vs tracing in every deployable
- severity: minor
- category: dependencies
- location: lib/cmd/Cargo.toml:20
- finding: brdgme_cmd hard-depends on `env_logger 0.11.11` (non-optional, even when the http-server feature is off), while bot/web/operator standardize on tracing + tracing-subscriber. Every game binary therefore ships the log-facade stack while the rest of the platform is on tracing. env_logger itself is maintained (0.11 is current); the issue is the split, and that a library crate initializes a logger implementation (an app-level concern).
- recommendation: move env_logger init behind the http-server feature or into the binaries, and consider tracing-subscriber with its env-filter (already in the tree) for consistency.

### num_cpus where std suffices
- severity: nit
- category: dependencies
- location: tools/fuzz/Cargo.toml:13
- finding: `num_cpus = "1.17.0"` duplicates `std::thread::available_parallelism()` (stable since Rust 1.59); the toolchain here is edition-2024-capable, so the crate is pure surplus for the single call site a fuzzer needs.
- recommendation: replace with `std::thread::available_parallelism()` and drop the dependency.

### lazy_static where std LazyLock suffices
- severity: nit
- category: dependencies
- location: lib/color/Cargo.toml:9 (also lords-of-vegas-1 per lock)
- finding: `lazy_static 1.5.0` is maintained-but-frozen and fully superseded by `std::sync::LazyLock` (stable since 1.80). Direct consumers: brdgme_color and lords-of-vegas-1; the remaining lock dependents are transitive.
- recommendation: migrate the two first-party consumers to LazyLock when convenient; the transitive copies stay regardless.

### convert_case at three versions
- severity: nit
- category: dependencies
- location: Cargo.lock (convert_case 0.6.0 via config/leptos_config, 0.10.0 via derive_more-impl, 0.11.0 via leptos_macro/server_fn_macro)
- finding: three copies of convert_case compile, all transitive (leptos config stack, derive_more, leptos macros). Small crate, build-time only; recorded for completeness as the only 3x duplicate outside the rand/getrandom cluster and hashbrown (4 versions, all transitive, standard ecosystem noise).
- recommendation: none actionable; will collapse as leptos and config converge.

### warp 0.4 for game-service HTTP while the rest of the platform is on axum
- severity: minor
- category: dependencies
- location: lib/cmd/Cargo.toml:17 (Cargo.lock:7355)
- finding: brdgme_cmd's http-server feature uses warp 0.4.3 while web, bot, and operator use axum 0.8 - two HTTP server frameworks in one workspace, and warp is compiled into all 28 game binaries. Mitigating: warp 0.4 shares the modern hyper 1 / http 1.4 stack with axum, so the marginal dependency cost is one framework layer, not a second HTTP stack. On maintenance: warp is a single-maintainer project (seanmonstar) with a much slower release cadence and less ecosystem investment than axum/tower; it is not unmaintained, but it is the weaker bet of the two, and keeping both means tracking two frameworks' upgrade cycles. Whether 0.4.3 is warp's latest should be confirmed.
- recommendation: consolidate the game-service HTTP layer onto axum 0.8 (the surface in lib/cmd is small - a couple of routes) and drop warp, eliminating the split rather than betting on warp's cadence.

### currency claims needing online confirmation
- severity: nit
- category: dependencies
- location: Cargo.lock (workspace-wide)
- finding: from knowledge (cutoff early 2026) the following look current-era but could not be verified offline: kube 4.0.0 / k8s-openapi 0.28 pairing, sentry 0.48.5, reqwest 0.13.4, rand 0.10.2, resend-rs 0.28, svix 1.98, mrml 6.0.1, mail-parser 0.11.5, async-nats 0.49.1, leptos-use 0.19 (against leptos 0.8), tower-sessions 0.14 / tower-sessions-sqlx-store 0.15, warp 0.4.3, axum-prometheus 0.10. None showed stale-major markers in the lock (no legacy http 0.2/hyper 0.14 edges except svix/actix noted above), but "is this the newest release" should be confirmed with `cargo outdated` or crates.io once network access is available.
- recommendation: run `cargo outdated --workspace` (or cargo-deny's advisory check) in CI as a scheduled job so currency drift is detected mechanically instead of by review.

## Areas reviewed and found clean
- Core stack currency: tokio 1.52, serde 1.0.228, serde_json 1.0.150, thiserror 2.0.18, anyhow 1.0.103, axum 0.8.9, leptos 0.8.20, tracing 0.1/tracing-subscriber 0.3, regex 1.13, uuid 1.23.5, time 0.3.53, log 0.4.33, wasm-bindgen 0.2.121, web-sys/js-sys 0.3.98, minijinja 2.21, aes-gcm 0.10.3 (current RustCrypto era), pulldown-cmark 0.13.4, dotenvy 0.15.7 (correctly chosen over abandoned dotenv), schemars 1.x, serial_test 3.5, hex 0.4, unicase 2.9, env_logger 0.11 (maintained; split flagged separately), petname 3.1, codee 0.3.5.
- Known-bad crates absent from the lock: atty, proc-macro-error, dotenv, instant, ansi_term, failure, net2, stdweb, wee_alloc, rustc-serialize, yaml-rust, derivative, owning_ref.
- thiserror 1.0.69 duplicate: transitive only (gloo-net 0.6 via leptos, tower-sessions-sqlx-store); all 40 first-party crates on 2.x.
- Duplicates judged normal ecosystem noise: hashbrown x4, windows-* families, bitflags x2, syn-era pairs (darling 0.20/0.23, itertools 0.14/0.15), http 0.2/1.4 (svix/actix only, flagged above), sha/digest/hmac RustCrypto pairs (sqlx 0.8/0.9 split, structure worker's scope), whoami/flume/webpki-roots/tungstenite pairs (same sqlx/async-nats era splits).
- No hand-rolled code found duplicating a solid off-the-shelf crate beyond items already excluded by charter (lib/game parser combinator, CommandSpec Parser impl).

## Tally
- critical: 0
- major: 2 (sentry default features pulling actix-web+ureq; unmaintained term_size)
- minor: 9 (combine dormancy, serde_yaml workspace view, paste advisory handling, triple rand stacks, web one-step-ahead pins, svix dual-http, chrono split, env_logger split, warp/axum split - warp counted here)
- nit: 4 (num_cpus, lazy_static, convert_case x3, currency-confirmation list)
