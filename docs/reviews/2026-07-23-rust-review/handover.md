# Rust code review — handover (2026-07-23)

Exhaustive, review-only audit of all Rust code in this repo. **No code changes
are to be made by any review unit.** This doc is the shared context for every
review Lead; the companion file `inventory.md` (same directory) holds the full
per-crate inventory (file counts, line counts, dependency lists, module maps)
and a shared-patterns writeup.

## Snapshot

All review work must run against the detached worktree snapshot, never the
main working tree (other agents modify it concurrently):

- **Worktree path:** `/home/beefsack/Development/brdgme-review-snapshot`
- **HEAD SHA:** `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`
  (verified identical via `git rev-parse HEAD` in both the main repo and the
  worktree at creation time)
- Rust workspace root inside the snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`

Rules:

- Review-only: no code edits anywhere, in either tree. The only allowed writes
  are markdown files under `docs/reviews/2026-07-23-rust-review/` in the MAIN
  repo (findings go there so they survive worktree removal).
- Never run workspace-wide cargo builds/tests. Targeted `cargo check -p
  <crate>` is permissible but generally unnecessary for review.
- Line numbers in findings must match the snapshot.

## Crate inventory (sizes)

39 workspace members, 355 `.rs` files, **113,077 total lines of Rust**.
Sorted by size descending; full details (deps, module maps) in `inventory.md`.

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

Distribution: `web` ~40% of LOC; 27 game crates ~49.6k; `lib/*` ~13.9k.

Key shared patterns (details in `inventory.md`):

- Uniform game-crate anatomy: `lib.rs` (state + `Gamer` impl + serde view
  structs), `command.rs` (combinator parser → Command enum), `render.rs`
  (`Renderer` impls emitting `brdgme_markup::Node`), plus game-specific
  modules. Each game crate also ships 4 near-identical boilerplate binaries
  (`_cli`, `_repl`, `_fuzz`, `_http`).
- Core contract: `Gamer` trait at `lib/game/src/game.rs:48`.
- Hand-rolled command parser combinator lives in
  `lib/game/src/command/parser/mod.rs` (deliberate — see exclusions below).
- Games are NOT linked into web/bot/operator; each game version is its own
  HTTP service reached via `lib/game_client` (`Host: {version}.games.internal`).
- web: Leptos 0.8 + axum 0.8 SSR on tokio; features `ssr` (server: sqlx,
  NATS JetStream, tower-sessions, resend/mrml email, sentry) vs `hydrate`
  (wasm frontend). bot: tokio daemon (NATS consumers, minijinja LLM prompts).
  operator: kube-rs 4 controller. tools/fuzz: random-command fuzzer driven by
  lib/rand_bot.
- No `[workspace.dependencies]`; shared versions are copy-pasted across 39
  manifests with known drift (sqlx 0.8 vs 0.9, getrandom 0.3 vs 0.4).

## Review criteria (verbatim — apply to every unit)

- Correctness - the code needs to do what it says.
- Quality - high quality, robust, reliable code.
- Simplicity - easy to read and follow, does one thing perfectly. Abstractions must earn their readability cost. Modules/types must not be oversized grab-bags of unrelated items, but also not over-split (filesystem complexity, diminishing returns).
- Consistency - consistent within the project and with library/framework community idioms. Hacks and workarounds should be aggressively flagged for replacement with idiomatic solutions.
- Dependencies - only popular, modern, well-maintained, documented, battle-hardened dependencies; lean modern; avoid bespoke solutions where an out-of-the-box one fits (notable exception: the custom serialisable command parser combinator in rust/lib/game, which is deliberate - do NOT flag it; also the duplicated `impl Parser for CommandSpec` in rust/lib/game/src/command/parser/mod.rs is deliberate, do not flag).

## Findings files

Each review unit writes its findings to
`docs/reviews/2026-07-23-rust-review/findings/<area>.md` in the MAIN repo
(e.g. `findings/games-batch-a.md`, `findings/web-server.md`). Every finding
uses this format:

```
### <short title>
- severity: critical | major | minor | nit
- category: correctness | quality | simplicity | consistency | dependencies
- location: path:line        (path relative to rust/, line numbers per the snapshot)
- finding: <what is wrong or notable, with enough context to act on>
- recommendation: <concrete suggested fix or direction>
```

Severity guide: critical = bug/data-loss/security risk; major = clear defect
or significant maintainability problem; minor = should fix, low urgency;
nit = polish. One `###` block per finding. If an area is clean, say so
explicitly in its file rather than leaving it absent.

## Recommended unit decomposition (13 units, sequential)

Sizes are snapshot LOC. Game batches group the 27 game crates by size so each
batch lands near 8–9.5k LOC.

1. **lib-game** — `lib/game` (3,737). Core trait, command parser/suggest
   engine, GameRng. Own unit per brief. → `findings/lib-game.md`
2. **lib-support** — `lib/cmd` (1,070), `lib/game_client` (738),
   `lib/markup` (2,709), `lib/color` (4,119), `lib/cost` (492),
   `lib/rand_bot` (142). ~9.3k. → `findings/lib-support.md`
3. **games-batch-a** — roll-through-the-ages-2 (4,936), starship-catan-1
   (4,046). ~9.0k. → `findings/games-batch-a.md`
4. **games-batch-b** — seven-wonders-1 (3,809), alhambra-1 (2,966),
   splendor-2 (2,719). ~9.5k. → `findings/games-batch-b.md`
5. **games-batch-c** — texas-holdem-2 (2,569), acquire-1 (2,520),
   cathedral-2 (2,256), sushizock-2 (2,078). ~9.4k. → `findings/games-batch-c.md`
6. **games-batch-d** — lords-of-vegas-1 (2,025), jaipur-2 (1,957),
   sushi-go-2 (1,923), modern-art-2 (1,700). ~7.6k. → `findings/games-batch-d.md`
7. **games-batch-e** — love-letter-2 (1,699), age-of-war-2 (1,656),
   lost-cities-2 (1,494), red7-1 (1,404), lost-cities-1 (1,342). ~7.6k.
   → `findings/games-batch-e.md`
8. **games-batch-f** — zombie-dice-2 (1,242), battleship-2 (1,236),
   for-sale-2 (1,108), category-5-2 (1,062), greed-2 (983), farkle-2 (881),
   tic-tac-toe-2 (786), no-thanks-2 (780), liars-dice-2 (728). ~8.0k.
   → `findings/games-batch-f.md`
9. **web-server** — web server infrastructure: `src/main.rs`, `router.rs`,
   `state.rs`, `config.rs`, `db.rs`, `nats.rs`, `error.rs`, `crypto.rs`,
   `admin.rs`, `auth/`, `websocket.rs`, `websocket_client.rs`,
   `src/bin/import_game.rs`. → `findings/web-server.md`
10. **web-domain** — web game/domain logic: `src/game/`, `src/game_info/`,
    `src/models/`, `src/stats/`, `players.rs`, `friends.rs`, `proposals.rs`,
    `new_game.rs`, `rules.rs`, `settings.rs`, `index.rs`.
    → `findings/web-domain.md`
11. **web-frontend-email** — `src/app.rs`, `src/lib.rs`, `src/components/`,
    `src/theme.rs`, `src/email/`. Read `docs/hydration.md` and `docs/email.md`
    first per AGENTS.md. → `findings/web-frontend-email.md`
    (Note: web units 9–11 split 45.6k LOC by module; exact per-module LOC
    split should be computed by the first web Lead from the snapshot, and if
    any single unit exceeds ~15k it should be split further.)
12. **bot-operator-tools** — `bot` (1,708), `operator` (412), `tools/fuzz`
    (358), `tools/render_plain` (32), `tools/repl` (10). ~2.5k.
    → `findings/bot-operator-tools.md`
13. **dependencies** — workspace-wide Cargo.toml/Cargo.lock audit: version
    drift (sqlx 0.8/0.9, getrandom 0.3/0.4), missing
    `[workspace.dependencies]` inheritance, per-crate dependency choices
    against the Dependencies criterion, the 108 boilerplate game binaries,
    `lib/cost` single-consumer duplication vs splendor-2's local cost module.
    → `findings/dependencies.md`

## Known non-issues (do not re-flag)

- Custom command parser combinator in `rust/lib/game` — deliberate.
- Duplicated `impl Parser for CommandSpec` in
  `rust/lib/game/src/command/parser/mod.rs` — deliberate, powers the suggest
  engine (see `docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md`).
- DB-backed tests failing in plain local runs — pre-existing condition
  (backlog #40); irrelevant to this review-only audit anyway.
