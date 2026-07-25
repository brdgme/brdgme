# Findings: dependencies (workspace-wide Cargo audit)

Scope: root `Cargo.toml` (65 ln, 40 members - the handover said 39; the
snapshot has 40), all 40 member `Cargo.toml` files (~998 ln), `Cargo.lock`
(709 packages, queried mechanically, never read wholesale), `deny.toml`
(83 ln), `rust-toolchain.toml`. Snapshot
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. Raw worker dumps and the
review log are in `findings/raw/dependencies-*.md`. Lead spot-checks
against the snapshot: zero `workspace.dependencies`/`workspace = true`
hits; sqlx 0.8.6 + 0.9.0 both in the lock; sentry 0.48.5's lock entry
lists sentry-actix, ureq and native-tls with all four sentry declarations
on default features; term_size 0.3.2 direct in lib/cmd.

Currency was assessed offline (no network per charter); knowledge-based
claims that need online confirmation are gathered in a dedicated finding.
One cross-worker duplicate (chrono in rand_bot) was merged in curation.
Cross-references to Unit 12 findings (serde_yaml in bot, num_cpus in
tools/fuzz, getrandom in bot/crypto) are noted inline.

## Workspace structure

### No [workspace.dependencies] - shared versions copy-pasted across 40 manifests
- severity: major
- category: dependencies
- location: Cargo.toml:1 (root; representative repeats: game/tic-tac-toe-2/Cargo.toml:14-16, lib/cmd/Cargo.toml:13-14)
- finding: The root manifest defines no `[workspace.dependencies]`; no member uses `workspace = true` (grep: zero hits). Shared deps are repeated per-manifest: serde 36 crates (34x "1.0.228" + 2x "1"), rand 33x "0.10.2", tokio 33 crates (27x "1.52.3" + 6x "1"), serde_json 19 crates in three spellings ("1.0.150" x13, "1.0" x4, "1" x2), thiserror 9 (7x "2.0.18" + 2x "2"), anyhow 4 (3x "1.0.103" + 1x "1"), axum 4 (2x "0.8.9" + 2x "0.8"), uuid 3, sentry 4x "0.48", reqwest 4x "0.13". A version bump of serde/tokio/rand touches 33-36 files; the mixed precise-vs-major spellings show edits already drifting.
- recommendation: Add `[workspace.dependencies]` in the root for every dep used by >1 crate and switch members to `dep.workspace = true` with per-crate `features = [...]` additions.

### No [workspace.package] - package metadata repeated and already inconsistent
- severity: minor
- category: consistency
- location: game/tic-tac-toe-2/Cargo.toml:2-6 (representative); bot/Cargo.toml:1-5
- finding: All 40 members repeat `version = "0.1.0"`, `publish = false`, `edition = "2024"` (40/40 each). `authors` is present in 37 manifests but missing from bot, web, operator - drift that `[workspace.package]` inheritance would have prevented. No manifest declares `license`/`repository` (acceptable for publish = false).
- recommendation: Add `[workspace.package]` (version, edition, publish, authors, license) to the root and replace the per-crate fields with `field.workspace = true`.

### No [workspace.lints] table
- severity: minor
- category: quality
- location: Cargo.toml:44 (end of [workspace] section)
- finding: Neither the root nor any member defines `[lints]`/`[workspace.lints]` (grep: zero hits). Clippy/rustc lint policy is whatever each developer's invocation passes and cannot be tightened workspace-wide (e.g. `unwrap_used`, `todo`) without touching 40 files later.
- recommendation: Add `[workspace.lints.rust]` / `[workspace.lints.clippy]` in the root plus `[lints] workspace = true` in members; pairs naturally with the workspace.dependencies migration.

### Duplicated [profile.wasm-release] in web/Cargo.toml is ignored by cargo
- severity: minor
- category: correctness
- location: web/Cargo.toml:143-148; Cargo.toml:56-62
- finding: web/Cargo.toml declares `[profile.wasm-release]`, but cargo only honours profiles in the workspace root manifest and emits "profiles for the non root package will be ignored" on every build; the copy that actually applies is Cargo.toml:56-62. The two are currently in sync except the root adds `debug = true` - exactly the silent-divergence failure mode.
- recommendation: Delete the profile block from web/Cargo.toml; keep only the root definition (leptos' `lib-profile-release = "wasm-release"` at web/Cargo.toml:177 resolves against the root profile).

### Root members list unsorted; unused custom profiles
- severity: nit
- category: consistency
- location: Cargo.toml:2-44; Cargo.toml:49-53
- finding: The `members` array is mostly-but-not-quite alphabetical (`age-of-war-2` after `alhambra-1`; `lost-cities-1` after `modern-art-2`; `roll-through-the-ages-2` after `seven-wonders-1`), inviting merge conflicts. `[profile.android-dev]` and `[profile.server-dev]` are empty `inherits = "dev"` shells with no reference anywhere in the repo's toml/json/nix files.
- recommendation: Sort members (or use `members = ["game/*", ...]` globs, removing the per-game root edit entirely); delete or document the two no-op profiles.

## Version drift and lockfile duplication

### sqlx split 0.8 (web) vs 0.9 (bot, operator) - two sqlx stacks compiled
- severity: major
- category: dependencies
- location: web/Cargo.toml:28; bot/Cargo.toml:16; operator/Cargo.toml:28
- finding: web pins `sqlx = "0.8"` (features `runtime-tokio-rustls`) while bot and operator use `"0.9"` (features `runtime-tokio` + `tls-rustls`, the 0.9 spelling). Cargo.lock confirms both sqlx 0.8.6 and 0.9.0 are built, duplicating the entire sqlx/sqlx-postgres stack in compile time and behaviour risk (two pools, two type-mapping behaviours against the same database). web is likely held on 0.8 by `tower-sessions-sqlx-store 0.15.0` (web/Cargo.toml:40).
- recommendation: Migrate web to sqlx 0.9 (bump tower-sessions-sqlx-store to an sqlx-0.9-compatible release, or vendor the trivial session-store impl), then move sqlx into `[workspace.dependencies]`.

### getrandom split 0.3 (bot) vs 0.4 (web); lock carries 0.2/0.3/0.4
- severity: minor
- category: dependencies
- location: bot/Cargo.toml:37; web/Cargo.toml:30
- finding: bot declares `getrandom = "0.3"`, web `getrandom = { version = "0.4", features = ["wasm_js"] }`. Cargo.lock contains getrandom 0.2.17, 0.3.4 and 0.4.3 - three copies of the RNG-source crate. (Unit 12 separately recommends dropping bot's direct getrandom entirely in favour of `aes-gcm`'s `generate_nonce`.)
- recommendation: Drop or bump bot's direct getrandom to 0.4; track residual transitive duplication under the bans policy.

### Three parallel rand stacks in the lock (0.8 / 0.9 / 0.10)
- severity: minor
- category: dependencies
- location: Cargo.lock (rand 0.8.7, 0.9.5, 0.10.2; rand_core and rand_chacha x3 each)
- finding: The workspace is uniformly on rand 0.10.2 (all first-party consumers), but three full rand stacks compile anyway: rand 0.9.5 via leptos, governor (axum-prometheus), sentry-core/types, metrics-util, tungstenite 0.29; rand 0.8.7 via nkeys/nuid (async-nats), num-bigint-dig, sqlx 0.8's drivers, tokio-websockets, tower-sessions-core. First-party code is clean; this is ecosystem-era lag among direct deps' own trees plus the sqlx 0.8/0.9 split. The largest build-bloat duplicate cluster after actix (below).
- recommendation: No first-party change possible for most; the sqlx-driven 0.8 copies disappear when the sqlx split is resolved. Re-check after the next leptos/async-nats upgrades.

### web pins tower-http 0.7 / gloo-net 0.7 / gloo-timers 0.4 one step ahead of their ecosystems, doubling each crate
- severity: minor
- category: dependencies
- location: web/Cargo.toml:41,75,76 (Cargo.lock: tower-http 0.6.11+0.7.0, gloo-net 0.6.0+0.7.0, gloo-timers 0.3.0+0.4.0)
- finding: web declares tower-http 0.7, but axum-prometheus, kube-client, leptos_axum, and reqwest all still consume tower-http 0.6.11, so both versions build. Same pattern for gloo-net (leptos_router/server_fn on 0.6, web on 0.7 - two copies in the WASM bundle, where size matters given the wasm-release `opt-level = "z"` effort) and gloo-timers (backon on 0.3). Being one minor ahead of leptos buys nothing and costs a duplicate compile of each; for gloo-net it plausibly costs WASM bytes.
- recommendation: Pin web to the versions its frameworks use (tower-http 0.6, gloo-net 0.6, gloo-timers 0.3) until leptos/reqwest move, or verify the newer APIs are actually required.

### chrono in rand_bot vs time everywhere else
- severity: minor
- category: consistency
- location: lib/rand_bot/Cargo.toml:11
- finding: brdgme_rand_bot is the only crate in the workspace using chrono (0.4.45, serde feature); every other datetime consumer (cmd, game, bot, web) standardizes on time 0.3. Two full date/time libraries compile for one consumer, and the workspace loses type-compatibility between rand_bot timestamps and everything else.
- recommendation: Port rand_bot's chrono usage to time 0.3 and drop chrono from the lock.

### Game-crate tokio uses features = ["full"]; feature-set drift on shared deps
- severity: minor
- category: dependencies
- location: game/tic-tac-toe-2/Cargo.toml:16 (identical line in all 27 game crates)
- finding: All 27 game crates declare `tokio = { version = "1.52.3", features = ["full"] }`, but their only async surface is the 13-line `*_http` bin calling `brdgme_cmd::http::serve` - `full` drags in fs/process/io-util/sync/etc. for every game build, 27 times over. Feature-set drift also exists generally: serde has `derive` in 35 declarations but not tools/fuzz; bot/operator/web each hand-pick different tokio feature sets.
- recommendation: Reduce game crates to `features = ["rt-multi-thread", "macros"]` (or let a shared bin crate own the tokio dep - see the boilerplate finding); centralise feature sets via `[workspace.dependencies]`.

## Dependency health and currency

### sentry default features drag actix-web 4 and ureq 3 into every server build
- severity: major
- category: dependencies
- location: bot/Cargo.toml:21 (also web/Cargo.toml:86, lib/cmd/Cargo.toml:19, lib/game_client/Cargo.toml:16)
- finding: All four sentry declarations are `sentry = "0.48"` with default features. The locked `sentry 0.48.5` package depends on `sentry-actix`, which pulls the full actix-web framework (8 `actix-*` packages in the lock), plus `ureq 3.3.0` (a third HTTP client alongside reqwest and hyper) with `native-tls`/`openssl`. The workspace serves HTTP with axum and warp only; actix is dead weight compiled into bot, web (ssr), operator (via game_client), and all 28 game binaries (via brdgme_cmd's http-server feature). The native-tls transport itself is documented as deliberate (web/Cargo.toml comment), but nothing uses actix or ureq.
- recommendation: Declare sentry with `default-features = false` and only the needed features (e.g. `backtrace`, `contexts`, `panic`, `debug-images`, `reqwest`, `native-tls`), ideally once via `[workspace.dependencies]`. Verify with `cargo tree -i actix-web` that the actix and ureq subtrees drop out of the lock.

### term_size 0.3.2 is unmaintained (RUSTSEC-2020-0163)
- severity: major
- category: dependencies
- location: lib/cmd/Cargo.toml:16
- finding: `term_size = "0.3.2"` is a direct dependency of brdgme_cmd, which every game binary and the repl link. term_size has been archived/unmaintained since 2020 and carries the RUSTSEC-2020-0163 unmaintained advisory (currently ignored in deny.toml:19-27); last release 2018. A direct violation of the "well-maintained, battle-hardened" charter.
- recommendation: Replace with `terminal_size` (the maintained successor, same one-call API) in lib/cmd and drop the deny.toml ignore.

### serde_yaml is deprecated/archived - workspace-level view
- severity: minor
- category: dependencies
- location: lib/game_client/Cargo.toml:15 (also bot/Cargo.toml:29)
- finding: The lock records `serde_yaml 0.9.34+deprecated`; the crate was archived in 2024 and its backend `unsafe-libyaml 0.2.11` is likewise archived. Unit 12 already flags the bot usage; workspace-wide there are exactly two direct consumers: bot and lib/game_client. Fixing only bot would leave the archived crate in the tree via game_client.
- recommendation: Migrate both consumers together to a maintained YAML crate (e.g. the saphyr family - kube already pulls `serde-saphyr` transitively) or to JSON if the YAML surface is internal-only.

### combine 4.6 parser library is dormant in the two core parsing crates
- severity: minor
- category: dependencies
- location: lib/game/Cargo.toml:12 and lib/markup/Cargo.toml:10
- finding: `combine = "4.6.7"` backs the brdgme_markup parser and parts of brdgme_game. combine's development has been effectively dormant for years (4.6.x is the terminal series; the announced 5.0 rewrite never shipped), and the ecosystem has consolidated on nom/winnow for actively maintained parser combinators. Not an advisory, but a low-momentum dependency at the heart of the markup pipeline. (The custom in-house parser combinator in lib/game is deliberate and not flagged; this is specifically the third-party combine dependency sitting next to it.)
- recommendation: No urgent action; when brdgme_markup is next touched substantially, consider migrating to winnow (or folding into the existing in-house combinator). Record as accepted risk otherwise.

### warp 0.4 for game-service HTTP while the rest of the platform is on axum
- severity: minor
- category: dependencies
- location: lib/cmd/Cargo.toml:17
- finding: brdgme_cmd's http-server feature uses warp 0.4.3 while web, bot, and operator use axum 0.8 - two HTTP server frameworks in one workspace, with warp compiled into all 28 game binaries. Mitigating: warp 0.4 shares the modern hyper 1 / http 1.4 stack with axum, so the marginal cost is one framework layer, not a second HTTP stack. On maintenance: warp is a single-maintainer project with a much slower release cadence and less ecosystem investment than axum/tower; not unmaintained, but the weaker bet of the two, and keeping both means tracking two frameworks' upgrade cycles.
- recommendation: Consolidate the game-service HTTP layer onto axum 0.8 (the surface in lib/cmd is a couple of routes) and drop warp.

### env_logger/log in brdgme_cmd vs tracing in every deployable
- severity: minor
- category: consistency
- location: lib/cmd/Cargo.toml:20
- finding: brdgme_cmd hard-depends on `env_logger 0.11.11` (non-optional, even when the http-server feature is off), while bot/web/operator standardize on tracing + tracing-subscriber. Every game binary ships the log-facade stack while the rest of the platform is on tracing. env_logger itself is maintained; the issue is the split, and that a library crate initializes a logger implementation (an app-level concern).
- recommendation: Move env_logger init behind the http-server feature or into the binaries, and consider tracing-subscriber with env-filter (already in the tree) for consistency.

### paste 1.0.15 (unmaintained, RUSTSEC-2024-0436) via the leptos stack
- severity: minor
- category: dependencies
- location: Cargo.lock (paste 1.0.15; dependents all leptos-family) pulled by web/Cargo.toml:19
- finding: `paste 1.0.15` carries the RUSTSEC-2024-0436 unmaintained advisory. It is purely transitive from the leptos 0.8 ecosystem (six dependents), so not actionable beyond acknowledgment - proc-macro-only, build-time, no runtime exposure. The other usual unmaintained suspects (atty, proc-macro-error, dotenv, instant, ansi_term) are all absent from the lock.
- recommendation: The existing deny.toml ignore for paste should carry a comment naming leptos as the source so the ignore list stays auditable; revisit when leptos drops paste.

### svix 1.98 pulls both http 0.2 and http 1.x
- severity: minor
- category: dependencies
- location: Cargo.lock (svix; pulled by web/Cargo.toml:50)
- finding: svix's lock entry depends on `http 0.2.12` and `http 1.4.2` simultaneously; svix and the actix stack (sentry finding above) are the only things keeping the legacy http 0.2 line in the tree. svix also duplicates itertools (0.15.0 vs the ecosystem's 0.14.0). Vendor SDK, so limited leverage, but it is the marker of the one remaining pre-hyper-1.0 corner of the lock.
- recommendation: Check whether a newer svix release drops the http 0.2 dependency at the next refresh; once the sentry/actix fix lands, svix will be the sole http 0.2 holdout.

### num_cpus where std suffices
- severity: nit
- category: dependencies
- location: tools/fuzz/Cargo.toml:13
- finding: `num_cpus = "1.17.0"` duplicates `std::thread::available_parallelism()` (stable since 1.59). Sole call site is the fuzzer's thread count (also flagged in Unit 12).
- recommendation: Replace with `std::thread::available_parallelism()` and drop the dependency.

### lazy_static where std LazyLock suffices
- severity: nit
- category: dependencies
- location: lib/color/Cargo.toml:9 (also game/lords-of-vegas-1 per the lock)
- finding: `lazy_static 1.5.0` is maintained-but-frozen and fully superseded by `std::sync::LazyLock` (stable since 1.80). Direct consumers: brdgme_color and lords-of-vegas-1; remaining lock dependents are transitive.
- recommendation: Migrate the two first-party consumers to LazyLock when convenient.

### convert_case at three versions
- severity: nit
- category: dependencies
- location: Cargo.lock (convert_case 0.6.0 via config/leptos_config, 0.10.0 via derive_more-impl, 0.11.0 via leptos macros)
- finding: Three copies of convert_case compile, all transitive. Small crate, build-time only; recorded as the only 3x duplicate outside the rand/getrandom cluster and hashbrown (4 versions, all transitive, standard ecosystem noise).
- recommendation: None actionable; will collapse as leptos and config converge.

### Currency claims needing online confirmation
- severity: nit
- category: dependencies
- location: Cargo.lock (workspace-wide)
- finding: From knowledge (cutoff early 2026) the following look current-era but could not be verified offline per the review's no-network constraint: kube 4.0.0 / k8s-openapi 0.28 pairing, sentry 0.48.5, reqwest 0.13.4, rand 0.10.2, resend-rs 0.28, svix 1.98, mrml 6.0.1, mail-parser 0.11.5, async-nats 0.49.1, leptos-use 0.19 (against leptos 0.8), tower-sessions 0.14 / tower-sessions-sqlx-store 0.15, warp 0.4.3, axum-prometheus 0.10. None showed stale-major markers in the lock (no legacy http 0.2/hyper 0.14 edges except svix/actix, flagged above).
- recommendation: Run `cargo outdated --workspace` (or a scheduled cargo-deny job) in CI so currency drift is detected mechanically instead of by review.

## Policy (deny.toml)

### deny.toml bans: multiple-versions only warns, so demonstrated duplicates never fail
- severity: minor
- category: dependencies
- location: deny.toml:69-70
- finding: `[bans] multiple-versions = "warn"` with empty `skip`/`skip-tree`, and `wildcards = "allow"`. With 3x rand, 3x getrandom, 2x sqlx already in the lock, warn-level output is noise that nobody gates on, and new duplicates land silently. `[sources] unknown-registry`/`unknown-git` are also `"warn"` (deny.toml:80-81), so a git or alternate-registry dependency would only warn.
- recommendation: Set `multiple-versions = "deny"` and enumerate the known duplicates in `skip`/`skip-tree`; set sources checks and `wildcards` to `"deny"` - no member uses a wildcard req today, so it is free.

### 4 of 7 advisory ignores are stale - target crates absent from Cargo.lock
- severity: minor
- category: dependencies
- location: deny.toml:19-27
- finding: The ignores for RUSTSEC-2024-0365, RUSTSEC-2026-0136, RUSTSEC-2026-0137 (diesel 1.4.8) and RUSTSEC-2021-0153 (encoding via `email`) all cite "legacy rust/api", but no `api` member exists in this workspace and `diesel`/`encoding` do not appear in Cargo.lock at all (grep count 0 for both). The remaining three ignores (paste, proc-macro-error2, term_size) are live. Stale ignores mask a future reintroduction of these advisories and cause unmatched-ignore warnings.
- recommendation: Delete the four diesel/encoding ignore entries.

## Bespoke / duplication

### 27 game crates x 4 boilerplate binaries = 108 near-identical files
- severity: minor
- category: simplicity
- location: game/tic-tac-toe-2/src/bin/tic_tac_toe_2_cli.rs:1 (representative; 108 files under game/*/src/bin/)
- finding: Every game crate ships `_cli`/`_repl`/`_fuzz`/`_http` bins (5-13 lines each, ~38 lines/crate, 108 files total) whose only variation is the crate name in `use <game>::Game;`. This also forces every game manifest to depend on brdgme_cmd, brdgme_fuzz and tokio purely for the bins, and adding a fifth entry-point means 27 new files.
- recommendation: Replace with a single `brdgme_game_bins!(Game)` macro in lib/cmd (or one generic bin crate parameterised by feature/env) generating all four mains; game manifests then shrink and the tokio/fuzz deps move to one place.

### lib/cost has one consumer while splendor-2 reimplements cost locally
- severity: minor
- category: simplicity
- location: lib/cost/Cargo.toml:1; game/splendor-2/src/cost.rs:1
- finding: `brdgme_cost` (492 ln) is depended on only by game/seven-wonders-1 (grep over all manifests: 2 hits, the crate itself and seven-wonders-1). Meanwhile game/splendor-2 carries its own 155-line `src/cost.rs` doing the same resource-cost bookkeeping. The shared crate failed at its one job of being shared.
- recommendation: Either fold lib/cost into seven-wonders-1 (it is not shared in practice) or port splendor-2 onto it; the half-shared state is the worst option.

## Areas reviewed and found clean

- rust-toolchain.toml: pinned channel 1.97.0 with rustfmt/clippy components and the wasm32-unknown-unknown target - matches the leptos/web build needs.
- deny.toml graph/licenses: `all-features = true` correctly set (with a good comment) so web's ssr-only deps are covered; license allowlist is a sane permissive-only set; `[licenses.private] ignore = true` correctly scoped.
- Exact pins `wasm-bindgen = "=0.2.121"` (leptos version-lock requirement) and `petname = "=3.1.0"` look deliberate; operator's duplicated dev-dep sqlx features are intentional and commented (#[sqlx::test] unification); web's hydrate/ssr feature gating internally consistent; resolver = "2" set explicitly.
- Core stack currency judged CLEAN: tokio 1.52, serde 1.0.228, serde_json 1.0.150, thiserror 2.0.18, anyhow 1.0.103, axum 0.8.9, leptos 0.8.20, tracing 0.1 / tracing-subscriber 0.3, regex 1.13, uuid 1.23.5, time 0.3.53, wasm-bindgen 0.2.121, web-sys/js-sys 0.3.98, minijinja 2.21, aes-gcm 0.10.3, pulldown-cmark 0.13.4, dotenvy 0.15.7 (correctly chosen over abandoned dotenv), schemars 1.x, serial_test 3.5, hex 0.4, unicase 2.9, env_logger 0.11 (maintained; the split is flagged, not the crate), petname 3.1, codee 0.3.5.
- Known-bad crates absent from the lock: atty, proc-macro-error, dotenv, instant, ansi_term, failure, net2, stdweb, wee_alloc, rustc-serialize, yaml-rust, derivative, owning_ref.
- thiserror 1.0.69 duplicate: transitive only; all 40 first-party crates on 2.x. Other duplicates judged normal ecosystem noise: hashbrown x4, windows-* families, bitflags x2, darling/itertools pairs, RustCrypto pairs from the sqlx split, whoami/flume/webpki-roots/tungstenite pairs.
- No hand-rolled first-party code found duplicating a solid off-the-shelf crate beyond items already excluded by charter (lib/game parser combinator, CommandSpec Parser impl).

## Tally

- Unit total: 0 critical, 4 major, 17 minor, 5 nit (26 findings)
- (Raw: W1 structure 2M/8m/2n, W2 currency 2M/9m/4n; one chrono
  duplicate merged in curation.)
