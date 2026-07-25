# Rust workspace inventory — 2026-07-23 review

Snapshot: git worktree `/home/beefsack/Development/brdgme-review-snapshot`, HEAD
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. All commands run against
`$SNAPSHOT/rust`. Read-only survey; no builds or tests were run.

Workspace: 39 member crates (see `rust/Cargo.toml`), `resolver = "2"`.
**No `[workspace.dependencies]` section exists** — every crate pins its own
versions. Every crate declares `edition = "2024"` individually in its
`Cargo.toml`. Total Rust LOC across all 39 crates: **113,077**.

## Summary table (sorted by lines, descending)

| Crate | Files | Lines |
|---|---:|---:|
| web | 55 | 45,645 |
| game/roll-through-the-ages-2 | 14 | 4,936 |
| lib/color | 4 | 4,119 |
| game/starship-catan-1 | 9 | 4,046 |
| game/seven-wonders-1 | 9 | 3,809 |
| lib/game | 9 | 3,737 |
| game/alhambra-1 | 9 | 2,966 |
| game/splendor-2 | 11 | 2,719 |
| lib/markup | 11 | 2,709 |
| game/texas-holdem-2 | 10 | 2,569 |
| game/acquire-1 | 11 | 2,520 |
| game/cathedral-2 | 11 | 2,256 |
| game/sushizock-2 | 8 | 2,078 |
| game/lords-of-vegas-1 | 12 | 2,025 |
| game/jaipur-2 | 8 | 1,957 |
| game/sushi-go-2 | 8 | 1,923 |
| bot | 6 | 1,708 |
| game/modern-art-2 | 9 | 1,700 |
| game/love-letter-2 | 9 | 1,699 |
| game/age-of-war-2 | 9 | 1,656 |
| game/lost-cities-2 | 9 | 1,494 |
| game/red7-1 | 9 | 1,404 |
| game/lost-cities-1 | 9 | 1,342 |
| game/zombie-dice-2 | 8 | 1,242 |
| game/battleship-2 | 8 | 1,236 |
| game/for-sale-2 | 8 | 1,108 |
| lib/cmd | 11 | 1,070 |
| game/category-5-2 | 8 | 1,062 |
| game/greed-2 | 8 | 983 |
| game/farkle-2 | 8 | 881 |
| game/tic-tac-toe-2 | 8 | 786 |
| game/no-thanks-2 | 8 | 780 |
| lib/game_client | 1 | 738 |
| game/liars-dice-2 | 8 | 728 |
| lib/cost | 1 | 492 |
| operator | 3 | 412 |
| tools/fuzz | 2 | 358 |
| lib/rand_bot | 2 | 142 |
| tools/render_plain | 1 | 32 |
| tools/repl | 1 | 10 |
| **Total** | **355** | **113,077** |

`web` alone is ~40% of the workspace LOC. All 27 `game/*` crates together are
~49,600 LOC. The seven `lib/*` crates are ~13,900 LOC.

---

## Per-crate inventory

Notes on conventions used below:

- "path →" marks a path-dependency on another workspace crate (internal graph).
- `[dev]` marks dev-dependencies.
- All game crates (`game/*`) share an almost identical dependency block; the
  common block is spelled out for the first crate and abbreviated as
  "standard game deps" afterwards, with only deviations listed.

### Standard game dependency block (applies to all 27 game/* crates unless noted)

- brdgme_cmd = path → `../../lib/cmd`
- brdgme_fuzz = path → `../../tools/fuzz`
- brdgme_color = path → `../../lib/color`
- brdgme_game = path → `../../lib/game`
- brdgme_markup = path → `../../lib/markup`
- rand = "0.10.2"
- serde = "1.0.228" (features: derive)
- tokio = "1.52.3" (features: full)
- [dev] brdgme_cmd = path → `../../lib/cmd` (features: test-support)

Standard game module layout (deviations noted per crate):

```
src/lib.rs       — game state, Gamer impl
src/command.rs   — command parser (brdgme_game::command::parser combinators)
src/render.rs    — Renderer impls for PubState/PlayerState
src/bin/<name>_cli.rs   — CLI binary
src/bin/<name>_repl.rs  — REPL binary
src/bin/<name>_http.rs  — HTTP game-service binary (brdgme_cmd::http::serve, warp)
src/bin/<name>_fuzz.rs  — fuzz binary
```

---

### bot

1. Path: `bot`
2. Files: 6
3. Lines: 1,708
4. Dependencies:
   - tokio "1" (rt-multi-thread, macros, signal); serde "1" (derive);
     serde_json "1"; reqwest "0.13" (json, rustls, no default features);
     sqlx "0.9" (runtime-tokio, tls-rustls, postgres, uuid, time, json);
     uuid "1" (serde, v4); anyhow "1"; tracing "0.1";
     tracing-subscriber "0.3" (env-filter); sentry "0.48";
     sentry-tracing "0.48"; minijinja "2"; serde_yaml "0.9";
     time "0.3" (serde); async-nats "0.49.1"; futures-util "0.3.32";
     axum "0.8"; aes-gcm "0.10"; thiserror "2"; hex "0.4"; getrandom "0.3"
   - brdgme_cmd = path → `../lib/cmd` (default-features = false)
   - brdgme_color = path → `../lib/color`
   - brdgme_game = path → `../lib/game`
   - brdgme_game_client = path → `../lib/game_client` (features: sentry)
   - brdgme_markup = path → `../lib/markup`
5. Module map:

```
src/main.rs     — binary entry (tokio); axum /healthz; NATS JetStream consumers
src/config.rs   — bot/provider config (some dead-code-allowed, test-exercised API)
src/crypto.rs   — aes-gcm encryption helpers
src/nats.rs     — BotCommandEvent / BotTurnEvent NATS wiring
src/prompt.rs   — LLM prompt rendering (minijinja), spec_to_yaml
src/routing.rs  — ProviderRouter across LLM providers
```

### game/acquire-1

1. Path: `game/acquire-1`
2. Files: 11
3. Lines: 2,520
4. Dependencies: standard game deps **plus** thiserror "2.0.18".
5. Module map: standard layout **plus** `src/board.rs`, `src/corp.rs`,
   `src/stats.rs`.

### game/alhambra-1

1. Path: `game/alhambra-1`
2. Files: 9
3. Lines: 2,966
4. Dependencies: standard game deps; [dev] also serde_json "1.0.150".
5. Module map: standard layout **plus** `src/card.rs`.

### game/age-of-war-2

1. Path: `game/age-of-war-2`
2. Files: 9
3. Lines: 1,656
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout **plus** `src/castle.rs`.

### game/battleship-2

1. Path: `game/battleship-2`
2. Files: 8
3. Lines: 1,236
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/cathedral-2

1. Path: `game/cathedral-2`
2. Files: 11
3. Lines: 2,256
4. Dependencies: standard game deps; [dev] also serde_json "1.0.150".
5. Module map: standard layout **plus** `src/loc.rs`, `src/piece.rs`,
   `src/tile.rs`.

### game/category-5-2

1. Path: `game/category-5-2`
2. Files: 8
3. Lines: 1,062
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/farkle-2

1. Path: `game/farkle-2`
2. Files: 8
3. Lines: 881
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/for-sale-2

1. Path: `game/for-sale-2`
2. Files: 8
3. Lines: 1,108
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/greed-2

1. Path: `game/greed-2`
2. Files: 8
3. Lines: 983
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/jaipur-2

1. Path: `game/jaipur-2`
2. Files: 8
3. Lines: 1,957
4. Dependencies: standard game deps; [dev] also serde_json "1.0.150".
5. Module map: standard layout, no extra modules.

### game/liars-dice-2

1. Path: `game/liars-dice-2`
2. Files: 8
3. Lines: 728
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/lords-of-vegas-1

1. Path: `game/lords-of-vegas-1`
2. Files: 12
3. Lines: 2,025
4. Dependencies: standard game deps **plus** thiserror "2.0.18",
   lazy_static "1.5.0", serde_json "1.0.150".
5. Module map: standard layout **plus** `src/board.rs`, `src/card.rs`,
   `src/casino.rs`, `src/tile.rs`.

### game/modern-art-2

1. Path: `game/modern-art-2`
2. Files: 9
3. Lines: 1,700
4. Dependencies: standard game deps; [dev] also serde_json "1.0".
5. Module map: standard layout **plus** `src/card.rs`.

### game/lost-cities-1

1. Path: `game/lost-cities-1`
2. Files: 9
3. Lines: 1,342
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout **plus** `src/card.rs`.

### game/lost-cities-2

1. Path: `game/lost-cities-2`
2. Files: 9
3. Lines: 1,494
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout **plus** `src/card.rs`.

### game/love-letter-2

1. Path: `game/love-letter-2`
2. Files: 9
3. Lines: 1,699
4. Dependencies: standard game deps; [dev] also serde_json "1.0".
5. Module map: standard layout **plus** `src/card.rs`.

### game/no-thanks-2

1. Path: `game/no-thanks-2`
2. Files: 8
3. Lines: 780
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/red7-1

1. Path: `game/red7-1`
2. Files: 9
3. Lines: 1,404
4. Dependencies: standard game deps; [dev] also serde_json "1.0.150".
5. Module map: standard layout **plus** `src/card.rs`.

### game/seven-wonders-1

1. Path: `game/seven-wonders-1`
2. Files: 9
3. Lines: 3,809
4. Dependencies: standard game deps **plus** brdgme_cost = path →
   `../../lib/cost` (the only game crate using lib/cost);
   [dev] also serde_json "1.0.150".
5. Module map: standard layout **plus** `src/card.rs`.

### game/roll-through-the-ages-2

1. Path: `game/roll-through-the-ages-2`
2. Files: 14
3. Lines: 4,936 (largest game crate)
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout **plus** `src/development.rs`, `src/dice.rs`,
   `src/good.rs`, `src/monument.rs`, `src/player_board.rs`, `src/take.rs`.

### game/splendor-2

1. Path: `game/splendor-2`
2. Files: 11
3. Lines: 2,719
4. Dependencies: standard game deps; [dev] also serde_json "1.0".
5. Module map: standard layout **plus** `src/card.rs`, `src/cost.rs`,
   `src/player_board.rs`. (Note: has its own local `cost.rs`, does not use
   `lib/cost`.)

### game/starship-catan-1

1. Path: `game/starship-catan-1`
2. Files: 9
3. Lines: 4,046
4. Dependencies: standard game deps; [dev] also serde_json "1.0.150".
5. Module map: standard layout **plus** `src/card.rs`.

### game/sushi-go-2

1. Path: `game/sushi-go-2`
2. Files: 8
3. Lines: 1,923
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/sushizock-2

1. Path: `game/sushizock-2`
2. Files: 8
3. Lines: 2,078
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### game/texas-holdem-2

1. Path: `game/texas-holdem-2`
2. Files: 10
3. Lines: 2,569
4. Dependencies: standard game deps; [dev] also serde_json "1.0".
5. Module map: standard layout **plus** `src/card.rs`, `src/poker.rs`.

### game/tic-tac-toe-2

1. Path: `game/tic-tac-toe-2`
2. Files: 8
3. Lines: 786 (smallest game crate; canonical minimal example)
4. Dependencies: standard game deps; [dev] also serde_json "1.0.150".
5. Module map: standard layout, no extra modules.

### game/zombie-dice-2

1. Path: `game/zombie-dice-2`
2. Files: 8
3. Lines: 1,242
4. Dependencies: standard game deps (no extras).
5. Module map: standard layout, no extra modules.

### lib/cmd (`brdgme_cmd`)

1. Path: `lib/cmd`
2. Files: 11
3. Lines: 1,070
4. Dependencies:
   - brdgme_color = path → `../color`; brdgme_game = path → `../game`;
     brdgme_markup = path → `../markup`
   - time "0.3" (serde, parsing, formatting, macros); thiserror "2.0.18";
     serde "1.0.228" (derive); serde_json "1.0.150"; term_size "0.3.2";
     warp "0.4.3" (server, optional); tokio "1" (signal, optional);
     sentry "0.48" (optional); env_logger "0.11.11"; rand "0.10.2"
   - Features: `default = ["http-server"]`;
     `http-server = ["warp", "tokio", "sentry"]`; `test-support = []`
5. Module map:

```
src/lib.rs           — re-exports; feature-gated http / test_support
src/api.rs           — Request/Response wire types for game services
src/cli.rs           — CLI runner
src/repl.rs          — interactive REPL
src/bot_cli.rs       — bot CLI protocol
src/http.rs          — warp HTTP server for game crates (http-server feature)
src/requester/mod.rs — Requester trait
src/requester/local.rs    — in-process requester
src/requester/gamer.rs    — Gamer-backed requester
src/requester/error.rs
src/test_support.rs  — test helpers (test-support feature)
```

### lib/color (`brdgme_color`)

1. Path: `lib/color`
2. Files: 4
3. Lines: 4,119 (mostly palette data in `palette.rs`)
4. Dependencies: lazy_static "1.5.0"; regex "1.12.4"; serde "1.0.228"
   (derive); thiserror "2.0.18". No workspace path deps.
5. Module map:

```
src/lib.rs      — color types, LIGHT palette etc.
src/css.rs      — CSS output
src/error.rs
src/palette.rs  — large static palette tables
```

### lib/cost (`brdgme_cost`)

1. Path: `lib/cost`
2. Files: 1
3. Lines: 492
4. Dependencies: serde "1.0.228" (derive). No workspace path deps.
5. Module map: single `src/lib.rs` (resource-cost type; only consumer is
   game/seven-wonders-1).

### lib/game (`brdgme_game`)

1. Path: `lib/game`
2. Files: 9 (plus `command/parser/` subdir)
3. Lines: 3,737
4. Dependencies:
   - brdgme_color = path → `../color`; brdgme_markup = path → `../markup`
   - time "0.3" (serde); combine "4.6.7"; thiserror "2.0.18"; log "0.4.33";
     rand "0.10.2"; rand_chacha "0.10" (serde); serde "1.0.228" (derive);
     unicase "2.9.0"; serde_json "1.0.150"
5. Module map:

```
src/lib.rs           — re-exports Gamer, Renderer, Log, Status, ...
src/game.rs          — Gamer trait, Renderer trait, Status, CommandResponse,
                       Stat, gen_placings
src/game_log.rs      — Log type, placings_log
src/errors.rs        — GameError
src/rng.rs           — GameRng (rand_chacha, serializable)
src/bot.rs           — Botter / Fuzzer traits, BotCommand
src/command/mod.rs   — Spec type
src/command/doc.rs   — Doc combinators (name_desc etc.)
src/command/suggest.rs  — suggest/autocomplete engine (1,218 lines)
src/command/parser/mod.rs — parser combinators (Token, Enum, Chain*, Map,
                       AfterSpace, ...; 1,488 lines) plus the deliberately
                       duplicated `impl Parser for Spec` advancement mechanism
src/command/parser/chain.rs — Chain2..ChainN tuple chains
```

### lib/game_client (`brdgme_game_client`)

1. Path: `lib/game_client`
2. Files: 1
3. Lines: 738
4. Dependencies:
   - anyhow "1.0.103"; reqwest "0.13" (json, rustls, no defaults);
     serde_json "1.0.150"; serde_yaml "0.9"; tokio "1" (time); tracing "0.1";
     rand "0.10.2"; sentry "0.48" (optional, feature `sentry`)
   - brdgme_cmd = path → `../cmd` (default-features = false)
   - brdgme_game = path → `../game`
   - [dev] axum "0.8.9"; tokio "1" (rt-multi-thread, macros, time, net)
5. Module map: single `src/lib.rs` — shared HTTP client for calling game
   services through the KEDA HTTP interceptor (sets
   `Host: {version_name}.games.internal`), with bounded retry for transport
   failures. All in-cluster callers (web, bot, operator) use this.

### lib/markup (`brdgme_markup`)

1. Path: `lib/markup`
2. Files: 11
3. Lines: 2,709
4. Dependencies:
   - brdgme_color = path → `../color`
   - combine "4.6.7"; serde "1.0.228" (derive); thiserror "2.0.18"
5. Module map:

```
src/lib.rs        — re-exports Node/TNode/Align/Row, html(), from_string, ...
src/ast.rs        — Node/TNode AST, table/row helpers
src/parser.rs     — combine-based `{{...}}` markup parser
src/ansi.rs       — ANSI terminal rendering
src/html.rs       — HTML rendering
src/html_class.rs — HTML class + CSS generation
src/plain.rs      — plain-text rendering
src/semantic.rs   — semantic player/color transform
src/transform.rs  — player-substitution transforms, line wrapping helpers
src/wrap.rs       — word wrap
src/error.rs      — MarkupError
```

### lib/rand_bot (`brdgme_rand_bot`)

1. Path: `lib/rand_bot`
2. Files: 2
3. Lines: 142
4. Dependencies:
   - brdgme_cmd = path → `../cmd`; brdgme_game = path → `../game`
   - chrono "0.4.45" (serde); rand "0.10.2"; serde_json "1.0.150"
5. Module map: `src/lib.rs` (random-command bot driven by command Spec) +
   `src/main.rs` (bot_cli binary wrapper).

### tools/fuzz (`brdgme_fuzz`)

1. Path: `tools/fuzz`
2. Files: 2
3. Lines: 358
4. Dependencies:
   - brdgme_cmd = path → `../../lib/cmd`; brdgme_game = path →
     `../../lib/game`; brdgme_rand_bot = path → `../../lib/rand_bot`
   - anyhow "1.0.103"; num_cpus "1.17.0"; rand "0.10.2"; serde "1.0.228"
5. Module map: `src/lib.rs` (multi-threaded fuzz runner driving a game via
   random commands from the spec, one thread per CPU) + `src/main.rs`
   (arg-parsing wrapper around `brdgme_cmd::requester`).

### tools/render_plain (`brdgme_render_plain`)

1. Path: `tools/render_plain`
2. Files: 1
3. Lines: 32
4. Dependencies: brdgme_color = path → `../../lib/color`;
   brdgme_markup = path → `../../lib/markup`.
5. Module map: single `src/main.rs` — renders a brdgme markup string from
   stdin to plain text, substituting player names from argv (works with both
   Rust and Go CLI output).

### tools/repl (`brdgme_repl`)

1. Path: `tools/repl`
2. Files: 1
3. Lines: 10
4. Dependencies: brdgme_cmd = path → `../../lib/cmd`.
5. Module map: single `src/main.rs` — thin binary over
   `brdgme_cmd::{repl, requester}`.

### web

1. Path: `web`
2. Files: 55
3. Lines: 45,645 (~40% of workspace)
4. Dependencies (heavily feature-gated; `[lib] crate-type = ["cdylib", "rlib"]`,
   features `ssr` / `hydrate` split server vs wasm builds):
   - UI: leptos "0.8.20", leptos_router "0.8.14", leptos_meta "0.8.6",
     leptos-use "0.19" (use_websocket, use_event_listener, use_document),
     leptos_axum "0.8.10" (ssr-only)
   - Server (ssr): axum "0.8.9" (ws, macros), tokio "1" (rt-multi-thread,
     macros, signal), tower "0.5" (util), tower-sessions "0.14.0",
     tower-sessions-sqlx-store "0.15.0" (postgres), tower-http "0.7" (cors,
     trace, limit, timeout), sqlx "0.8" (runtime-tokio-rustls, postgres,
     uuid, migrate), axum-prometheus "0.10.0"
   - Browser (hydrate): wasm-bindgen "=0.2.121", console_error_panic_hook
     "0.1", gloo-net "0.7" (websocket), gloo-timers "0.4" (futures),
     web-sys "0.3.77", js-sys "0.3", codee "0.3.5"
   - Email/auth/misc: resend-rs "0.28" (rustls-tls), mrml "6.0.1",
     svix "1.98", mail-parser "0.11", pulldown-cmark "0.13.4",
     petname "=3.1.0", aes-gcm "0.10", hex "0.4", async-nats "0.49.1",
     async-trait "0.1", dotenvy "0.15", anyhow "1.0.103",
     thiserror "2.0.18", futures-util "0.3.32", rand "0.10.2",
     getrandom "0.4" (wasm_js), uuid "1.23.4" (serde, v4, js),
     time "0.3", serde "1.0.228", serde_json "1.0.150", reqwest "0.13"
     (json, form, rustls), tracing/tracing-subscriber, sentry "0.48" +
     sentry-tower + sentry-tracing
   - Workspace: brdgme_cmd (path → `../lib/cmd`, no default features),
     brdgme_game (path → `../lib/game`), brdgme_color (path →
     `../lib/color`), brdgme_markup (path → `../lib/markup`),
     brdgme_game_client (path → `../lib/game_client`, features sentry) —
     all ssr-optional. **web does not depend on any game/* crate.**
   - [dev] serial_test "3.5.0", tokio-tungstenite "0.30"
   - Extra bin: `import-game` (`src/bin/import_game.rs`, requires ssr)
   - `[package.metadata.leptos]` cargo-leptos config (site root, wasm profile,
     `style/main.scss`, `public/` assets)
5. Module map (one/two levels):

```
src/main.rs        — #[tokio::main] ssr entry: DB pool, NATS JetStream,
                     GameBroadcaster, resend, sentry, axum router
src/lib.rs         — hydration entry (wasm)
src/app.rs         — Leptos root app / routes
src/router.rs      — axum router assembly
src/state.rs       — AppState
src/config.rs, src/error.rs, src/theme.rs, src/crypto.rs, src/db.rs,
src/nats.rs, src/index.rs, src/admin.rs, src/friends.rs, src/players.rs,
src/proposals.rs, src/rules.rs, src/settings.rs, src/new_game.rs,
src/websocket.rs, src/websocket_client.rs
src/auth/          — mod.rs, server.rs, session.rs, blocked_domains.rs
src/components/    — mod.rs, game.rs, layout.rs, form.rs, confirm.rs,
                     opponent_slot.rs, spinner.rs (Leptos components)
src/email/         — mod.rs, inbound.rs, outbound.rs, notify.rs, render.rs,
                     commands.rs, sweep.rs
src/game/          — mod.rs (status_fields, broadcast_and_trigger,
                     execute_command), server_fns.rs, export.rs, import.rs
src/game_info/     — mod.rs, queries.rs
src/models/        — mod.rs, game.rs, user.rs
src/stats/         — mod.rs, queries.rs, viz.rs
src/bin/import_game.rs
```

### operator (`brdgme-operator`)

1. Path: `operator`
2. Files: 3
3. Lines: 412
4. Dependencies:
   - kube "4" (runtime, derive, client); k8s-openapi "0.28" (latest);
     schemars "1"; axum "0.8"; futures "0.3"; tokio "1" (rt-multi-thread,
     macros); serde "1" (derive); serde_json "1"; sqlx "0.9"
     (runtime-tokio, tls-rustls, postgres, uuid); uuid "1" (serde);
     reqwest "0.13" (json, rustls); rustls "0.23" (aws-lc-rs);
     thiserror "2"; tracing "0.1"; tracing-subscriber "0.3" (env-filter)
   - brdgme_cmd = path → `../lib/cmd` (default-features = false)
   - brdgme_game_client = path → `../lib/game_client`
   - [dev] sqlx "0.9" (+ macros, migrate)
5. Module map:

```
src/main.rs        — tokio entry, rustls aws-lc-rs provider install, axum
                     /healthz
src/controller.rs  — kube runtime controller/reconciler
src/crd.rs         — CustomResourceDefinition types (schemars/derive)
```

---

## Shared patterns (for reviewers)

### Edition & dependency management

- All 39 crates use **edition 2024**, declared per-crate; there is **no
  `[workspace.dependencies]`** and no workspace-level lints table. Shared
  versions (rand 0.10.2, serde 1.0.228, thiserror 2.0.18, tokio, serde_json
  1.0.150) are copy-pasted across manifests — version drift risk lives here.
- Notable version skew already present: `sqlx` is **0.8 in web** but **0.9 in
  bot and operator**; `getrandom` is 0.3 (bot) vs 0.4 (web); serde_json dev-dep
  pins vary between "1.0", "1.0.150" across game crates.

### Game crate anatomy (uniform across all 27 game/* crates)

- Every game crate has the same skeleton: `lib.rs` holds the `Game` state
  struct (serde `Serialize`/`Deserialize`, plus separate `PubState` and
  `PlayerState` view structs), an inherent impl with rule methods, and
  `impl brdgme_game::Gamer for Game`. `command.rs` builds a command parser
  from `brdgme_game::command::parser` combinators (`Doc::name_desc`,
  `Token`, `Enum::exact`, `Chain2`, `Map`, `AfterSpace`) mapping input to a
  `Command` enum. `render.rs` implements `brdgme_game::Renderer` (returns
  `Vec<brdgme_markup::Node>`) for both `PubState` and `PlayerState`.
- Every game crate ships **four identical boilerplate binaries** under
  `src/bin/`: `_cli`, `_repl`, `_fuzz`, and `_http` (the `_http` one is a
  ~12-line `brdgme_cmd::http::serve::<Game>(addr)` warp server). That is
  ~108 near-identical small files — a macro or codegen candidate.
- The `Gamer` trait (`lib/game/src/game.rs:48`) is the core contract:
  associated types `PubState`/`PlayerState: Serialize + DeserializeOwned +
  Renderer`; methods `start(players, seed)`, `pub_state`, `player_state`,
  `command(player, input, players) -> CommandResponse`, `status() ->
  Status::{Active, Finished}`, `command_spec`, `player_count(s)`, plus
  default helpers (`whose_turn`, `placings`, `points`, `rules`,
  `basic_strategy` etc.).
- Deterministic RNG: games hold a serializable `GameRng` (rand_chacha) in
  state so `start(players, seed)` is reproducible — important for
  replay/import/export correctness.
- Logs are `brdgme_markup::Node` trees (`Log::public(vec![N::Player(p),
  N::text(...), ...])`) — rendering is markup-first, with plain/ANSI/HTML
  output handled downstream by lib/markup.

### Command parser & suggest engine (lib/game)

- `lib/game/src/command/parser/mod.rs` (1,488 lines) is a hand-rolled
  combinator library (`Parser` trait with `parse` returning remaining input
  + completions). It contains the **deliberately duplicated
  `impl Parser for Spec`** (per AGENTS.md, ~lines 813–1040) that powers the
  suggest engine's advancement — documented as NOT dead code; do not propose
  removing it (see `docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md`).
- `command/suggest.rs` (1,218 lines) is the autocomplete/suggest engine built
  on the same parser infra.
- The `combine` crate (4.6.7) is used in lib/game and lib/markup, but the
  command parser itself is custom — two parsing styles coexist.

### Service topology (how games are wired in)

- Game crates do **not** get linked into web/bot/operator. Each game version
  runs as its own HTTP service (the `_http` binaries, warp via
  `brdgme_cmd::http`), and all in-cluster callers reach it through
  **lib/game_client**, which sets `Host: {version_name}.games.internal` for
  the KEDA HTTP interceptor and applies a bounded transport-retry policy.
- `web` is a Leptos 0.8 + axum 0.8 SSR app (tokio runtime,
  `#[tokio::main]` in `src/main.rs`), split by cargo features: `ssr`
  (server: sqlx/Postgres, NATS JetStream, tower-sessions, resend email,
  sentry, prometheus) and `hydrate` (wasm client: gloo-*, web-sys,
  leptos-use websockets). `[lib] crate-type = ["cdylib", "rlib"]`.
- Real-time updates flow over NATS: `web/src/websocket.rs` GameBroadcaster
  pushes skinny game-update signals to browser websockets; bot turns are
  triggered via JetStream `bot.turn` events (`web/src/game/mod.rs`
  `broadcast_and_trigger` / `trigger_bot_turns`).
- `bot` is a tokio daemon (axum /healthz + NATS consumers) that renders LLM
  prompts with minijinja, routes across providers (`routing.rs`), and calls
  game services via brdgme_game_client.
- `operator` is a kube-rs (kube 4 / k8s-openapi 0.28) controller with a CRD
  (`crd.rs`) and reconciler (`controller.rs`), rustls aws-lc-rs, axum
  /healthz; it also talks to game services via brdgme_game_client and to
  Postgres via sqlx 0.9.

### Misc patterns worth knowing

- lib/cmd is the shared "game service framework": `api.rs` defines the
  Request/Response wire format, `http.rs` the warp server, `cli.rs`/`repl.rs`
  the human interfaces, `requester/` the abstraction used by fuzz/repl
  tooling. Features: default `http-server` (warp+tokio+sentry);
  `test-support` (enabled as a dev-dep by every game crate).
- tools/fuzz parallelizes one thread per CPU (num_cpus) driving random
  commands from lib/rand_bot against a requester — the fuzz binaries in each
  game crate are 6-line wrappers around it.
- tools/render_plain explicitly interoperates with the Go (`brdgme-go`)
  CLI output — the `{{...}}` markup syntax is a cross-language contract.
- `lib/cost` has exactly one consumer (seven-wonders-1) while splendor-2
  keeps its own local `cost.rs` — minor duplication to be aware of, not
  necessarily worth unifying.
- sentry/tracing instrumentation is uniform across the three daemons
  (web, bot, operator): tracing + tracing-subscriber env-filter +
  sentry/sentry-tracing 0.48.
