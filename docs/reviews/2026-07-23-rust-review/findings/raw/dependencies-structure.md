# Raw findings: dependency structure (Unit 13 W1)

Scope: workspace/manifest structure at snapshot f8763a5 - root `Cargo.toml` (65 ln, 40 members), all 40 member `Cargo.toml` files, `deny.toml` (83 ln), `rust-toolchain.toml`. Mechanical dep=version extraction across all manifests; `Cargo.lock` grepped only for specific packages.

### No [workspace.dependencies] - shared versions copy-pasted across 40 manifests
- severity: major
- category: dependencies
- location: Cargo.toml:1 (root; representative repeats: game/tic-tac-toe-2/Cargo.toml:14-16, lib/cmd/Cargo.toml:13-14)
- finding: The root manifest defines no `[workspace.dependencies]`; no member uses `workspace = true` (grep: zero hits). Shared deps are repeated per-manifest: serde 36 crates (34x "1.0.228" + 2x "1"), rand 33x "0.10.2", tokio 33 crates (27x "1.52.3" + 6x "1"), serde_json 19 crates in three spellings ("1.0.150" x13, "1.0" x4, "1" x2), thiserror 9 (7x "2.0.18" + 2x "2"), anyhow 4 (3x "1.0.103" + 1x "1"), axum 4 (2x "0.8.9" + 2x "0.8"), uuid 3 ("1.23.4" + 2x "1"), sentry 4x "0.48", reqwest 4x "0.13". A version bump of serde/tokio/rand touches 33-36 files; the mixed precise-vs-major spellings show edits already drifting.
- recommendation: Add `[workspace.dependencies]` in the root for every dep used by >1 crate (serde, serde_json, tokio, rand, thiserror, anyhow, axum, reqwest, sentry*, tracing*, uuid, time, sqlx, getrandom, futures-util, async-nats, aes-gcm, hex, serde_yaml) and switch members to `dep.workspace = true` with per-crate `features = [...]` additions.

### sqlx split 0.8 (web) vs 0.9 (bot, operator) - two sqlx stacks compiled
- severity: major
- category: dependencies
- location: web/Cargo.toml:28; bot/Cargo.toml:16; operator/Cargo.toml:28
- finding: web pins `sqlx = "0.8"` (features `runtime-tokio-rustls`) while bot and operator use `"0.9"` (features `runtime-tokio` + `tls-rustls`, the 0.9 spelling). Cargo.lock confirms both sqlx 0.8.6 and 0.9.0 are built (Cargo.lock:5978/5991), duplicating the entire sqlx/sqlx-postgres stack in compile time and binary risk (two pools, two type-mapping behaviours against the same database). web is likely held on 0.8 by `tower-sessions-sqlx-store 0.15.0` (web/Cargo.toml:40).
- recommendation: Migrate web to sqlx 0.9 (bump tower-sessions-sqlx-store to an sqlx-0.9-compatible release, or vendor the trivial session-store impl), then move sqlx into `[workspace.dependencies]`.

### getrandom split 0.3 (bot) vs 0.4 (web); lock carries 0.2/0.3/0.4
- severity: minor
- category: dependencies
- location: bot/Cargo.toml:37; web/Cargo.toml:30
- finding: bot declares `getrandom = "0.3"`, web `getrandom = { version = "0.4", features = ["wasm_js"] }`. Cargo.lock contains getrandom 0.2.17, 0.3.4 and 0.4.3 (Cargo.lock:2111/2124/2138) - three copies of the RNG-source crate. Similarly rand resolves to three versions: 0.8.7, 0.9.5, 0.10.2 (Cargo.lock:4742/4753/4763) via transitive deps.
- recommendation: Bump bot's direct getrandom to 0.4 (it exists only to pull an RNG backend; the direct-dep drift is fixable even though 0.2/0.3 remain transitively). Track the rand triplication under the bans policy (next finding).

### deny.toml bans: multiple-versions only warns, so demonstrated duplicates never fail
- severity: minor
- category: dependencies
- location: deny.toml:69-70
- finding: `[bans] multiple-versions = "warn"` with empty `skip`/`skip-tree`, and `wildcards = "allow"`. With 3x rand, 3x getrandom, 2x sqlx already in the lock, warn-level output is noise that nobody gates on, and new duplicates land silently. `[sources] unknown-registry`/`unknown-git` are also `"warn"` (deny.toml:80-81), so a git or alternate-registry dependency would only warn.
- recommendation: Set `multiple-versions = "deny"` and enumerate the known duplicates in `skip`/`skip-tree` (rand 0.8/0.9, getrandom 0.2/0.3, sqlx 0.8 until the web migration lands); set sources checks and `wildcards` to `"deny"` - no member uses a wildcard req today, so it is free.

### 4 of 7 advisory ignores are stale - target crates absent from Cargo.lock
- severity: minor
- category: dependencies
- location: deny.toml:19-27
- finding: The ignores for RUSTSEC-2024-0365, RUSTSEC-2026-0136, RUSTSEC-2026-0137 (diesel 1.4.8) and RUSTSEC-2021-0153 (encoding via `email`) all cite "legacy rust/api", but no `api` member exists in this workspace and `diesel`/`encoding` do not appear in Cargo.lock at all (grep count 0 for both). The remaining three ignores (paste, proc-macro-error2, term_size) are live. Stale ignores mask a future reintroduction of these advisories and make `cargo deny` emit unmatched-ignore warnings.
- recommendation: Delete the four diesel/encoding ignore entries (restore them only if/where rust/api is actually checked).

### No [workspace.package] - package metadata repeated and already inconsistent
- severity: minor
- category: consistency
- location: game/tic-tac-toe-2/Cargo.toml:2-6 (representative); bot/Cargo.toml:1-5
- finding: All 40 members repeat `version = "0.1.0"`, `publish = false`, `edition = "2024"` (40/40 each, verified by grep). `authors` is present in 37 manifests but missing from bot, web, operator - drift that `[workspace.package]` inheritance would have prevented. No manifest declares `license`/`repository` (acceptable for publish = false, but a one-line workspace-level `license` would cover it).
- recommendation: Add `[workspace.package]` (version, edition, publish, authors, license) to the root and replace the per-crate fields with `field.workspace = true`.

### No [workspace.lints] table
- severity: minor
- category: quality
- location: Cargo.toml:44 (end of [workspace] section)
- finding: Neither the root nor any member defines `[lints]`/`[workspace.lints]` (grep: zero hits). Clippy/rustc lint policy is therefore whatever each developer's invocation passes, and cannot be tightened workspace-wide (e.g. `unwrap_used`, `todo`, `rust_2018_idioms`) without touching 40 files later.
- recommendation: Add `[workspace.lints.rust]` / `[workspace.lints.clippy]` in the root plus `[lints] workspace = true` in members (the members edit is mechanical and pairs naturally with the workspace.dependencies migration).

### Duplicated [profile.wasm-release] in web/Cargo.toml is ignored by cargo
- severity: minor
- category: correctness
- location: web/Cargo.toml:143-148; Cargo.toml:56-62
- finding: web/Cargo.toml declares `[profile.wasm-release]`, but cargo only honours profiles in the workspace root manifest and emits "profiles for the non root package will be ignored" on every build; the copy that actually applies is Cargo.toml:56-62. The two are currently in sync except the root adds `debug = true` - exactly the silent-divergence failure mode.
- recommendation: Delete the profile block from web/Cargo.toml; keep only the root definition (leptos' `lib-profile-release = "wasm-release"` at web/Cargo.toml:177 resolves against the root profile).

### Game-crate tokio uses features = ["full"] for a one-line #[tokio::main]
- severity: minor
- category: dependencies
- location: game/tic-tac-toe-2/Cargo.toml:16 (identical line in all 27 game crates)
- finding: All 27 game crates declare `tokio = { version = "1.52.3", features = ["full"] }`, but their only async surface is the 13-line `*_http` bin calling `brdgme_cmd::http::serve` - lib/cmd itself needs only `signal` (lib/cmd/Cargo.toml:17) and warp brings its own runtime features. `full` drags in fs/process/io-util/sync/etc. for every game build, 27 times over. Feature-set drift also exists on shared deps generally: serde has `derive` in 35 declarations but not tools/fuzz; bot/operator/web each hand-pick different tokio feature sets ("rt-multi-thread, macros, signal" vs "rt-multi-thread, macros").
- recommendation: Reduce game crates to `features = ["rt-multi-thread", "macros"]` (or let a shared bin crate own the tokio dep entirely - see next finding); centralise feature sets via `[workspace.dependencies]`.

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
- recommendation: Either fold lib/cost into seven-wonders-1 (it is not shared in practice) or port splendor-2 onto it - pick one; the half-shared state is the worst option.

### rand_bot uses chrono while the rest of the workspace uses time
- severity: nit
- category: consistency
- location: lib/rand_bot/Cargo.toml:11
- finding: lib/rand_bot is the sole crate depending on `chrono = "0.4.45"`; lib/cmd, web, and bot all use `time = "0.3"`. Two datetime stacks compiled for one crate's usage.
- recommendation: Port rand_bot to `time` and drop chrono.

### Root members list unsorted; unused custom profiles
- severity: nit
- category: consistency
- location: Cargo.toml:2-44; Cargo.toml:49-53
- finding: The `members` array is mostly-but-not-quite alphabetical (`age-of-war-2` after `alhambra-1`; `lost-cities-1` after `modern-art-2`; `roll-through-the-ages-2` after `seven-wonders-1`), inviting merge conflicts and duplicate adds. `[profile.android-dev]` and `[profile.server-dev]` are empty `inherits = "dev"` shells with no reference anywhere in the repo's toml/json/nix files (grep: only their own definitions) - if they exist for external build scripts, a comment should say so.
- recommendation: Sort members (or use `members = ["game/*", ...]` globs, which removes the per-game root edit entirely); delete or document the two no-op profiles.

## Areas reviewed and found clean

- rust-toolchain.toml: pinned channel 1.97.0 with rustfmt/clippy components and the wasm32-unknown-unknown target - matches the leptos/web build needs; nothing to flag.
- deny.toml graph/licenses: `all-features = true` is correctly set (with a good explanatory comment) so web's ssr-only server deps are covered; license allowlist is a sane permissive-only set with `confidence-threshold = 0.8`, and `[licenses.private] ignore = true` is correctly scoped to internal publish = false crates.
- Exact pins `wasm-bindgen = "=0.2.121"` (leptos version-lock requirement) and `petname = "=3.1.0"` look deliberate.
- operator's duplicated dev-dependency sqlx with extra macros/migrate features is intentional and commented (feature unification for #[sqlx::test]).
- web feature gating (hydrate/ssr optional-dep wiring, required-features on the import-game bin) is internally consistent; ssr-only heavy deps (mrml, svix, sentry) are correctly optional.
- resolver = "2" is set explicitly at the root.
- Custom parser combinator in lib/game: out of scope by instruction (deliberate).

## Tally

- critical: 0
- major: 2
- minor: 8
- nit: 2
