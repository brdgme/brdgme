# T3-B7: bot service quality + web cargo deps

- **Batch**: T3-B7 = WP-61 (bot service quality, 12 findings) + WP-43 (web
  cargo deps, 5 findings)
- **Crates**: `rust/bot`, `rust/web`
- **Sources**: `findings/bot-operator-tools.md` (`bo Fnn`) and
  `findings/dependencies.md` (`dp Fnn`) - **no verification file exists for
  either; the raw files are authoritative**. `findings/web-server.md`
  (`ws Fnn`) is **superseded by `findings/verification/web-server.md`**, whose
  severities and verdicts are honoured below.
- **Numbering**: neither findings file carries inline ids. `Fnn` = the nth
  `###` heading in that file, counted top to bottom (both files' own header
  counts are off by one - `dependencies.md` says 26 with 27 headings,
  `bot-operator-tools.md` says 30 with 31 - but sequential numbering is
  anchored by dp F6/F12/F20 and bo F18/F25/F26/F28 and resolves correctly).
  Every id below was checked against the package's declared paths and matched.
- **Rows**: 17 (10 minor / 7 nit). One review-wide rejection pair (`d F13`,
  `ws F30`) touches neither package. `ws F67` is **UNVERIFIABLE, not
  rejected** - it rests on crates.io state outside the snapshot - and is kept
  as a row, flagged in place.
- **One edit, two rows**: `bo F12` (use aes-gcm's `generate_nonce`) also
  resolves `dp F7` (getrandom 0.3-vs-0.4 drift). They are two rows pointing at
  the same single edit in `rust/bot`; land them together.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by crate then source file so one session sweeps a file at a
time. Note that another agent has been landing WP-38/WP-39 fixes in
`rust/bot/src/main.rs`; if a row is already fixed, skip it and say so.

## WP-61 - bot (`src/main.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F4` | `rust/bot/src/main.rs` fn `merge_json_patch` | When the patch value is an object and the target entry is not, recurse against a fresh empty object per RFC 7396 (so nested nulls are stripped rather than cloned through) | y |
| `bo F7` | `rust/bot/src/main.rs` fn `run_bot_turn`, the `"Rendered prompt"` trace event | Log the user message too (or instead) - the event currently records only `messages[0]`, the static system prompt | n |

## WP-61 - bot (`src/main.rs` + `user_prompt.md`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F6` | `rust/bot/src/main.rs` fn `build_messages` + `rust/bot/user_prompt.md` player list | Pass a per-player `is_me` flag (computed from `player_position`, which the Rust side already holds) and key the `(you)` marker on it instead of `player.name == my_name` | y |

## WP-61 - bot (`src/config.rs` + `src/main.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F9` | `rust/bot/src/config.rs` fns `load_bot_config` / `bots_table_empty` / `load_providers` + `rust/bot/src/main.rs` fns `run_bot_turn` / `load_bot_context` / `build_messages` | Per `try_get(...).unwrap_or(default)` site: use `try_get::<Option<T>, _>` + `unwrap_or` where the column is nullable, and `.context(...)?` where it is NOT NULL, so decode/schema errors stop masquerading as data | y |

## WP-61 - bot (`src/config.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F10` | `rust/bot/src/config.rs` fn `env_fallback_provider` (the `LLM_EXTRA_BODY` read) | Log a warning when `LLM_EXTRA_BODY` is set but unparseable instead of `.ok()`-ing it away, and update test fn `env_fallback_provider_invalid_extra_body_is_none` to match | y |

## WP-61 - bot (`src/crypto.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F11` | `rust/bot/src/crypto.rs` fns `load_key` / `using_default_key` / `default_key` (caller: `main.rs` fn `main`) | Have `load_key` return a `Key::Default`/`Key::FromEnv`-style enum so the insecure-default warning derives from one env read instead of two independent ones | y |
| `bo F12` | `rust/bot/src/crypto.rs` fn `rand_nonce` (callers: fns `encrypt`) | Replace the hand-rolled 12-byte `getrandom` helper with `Aes256Gcm::generate_nonce` (`aes-gcm`'s `aead/os_rng` feature) and delete `rand_nonce` - **same edit as `dp F7` below** | n |

## WP-61 - bot (`src/prompt.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F13` | `rust/bot/src/prompt.rs` fn `markup_resolve_players` | Replace the sequential `str::replace` of `{{player N}}` with `brdgme_markup::from_string` + `transform` (already a declared dependency), removing the re-substitution quirk when a name contains a literal player tag | y |
| `bo F15` | `rust/bot/src/prompt.rs` fns `render_system` / `render_user` | Build the minijinja `Environment` once in a `static LazyLock` and reuse it instead of re-parsing the embedded templates on every render | n |

## WP-61 - bot (`src/routing.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F14` | `rust/bot/src/routing.rs` fns `ProviderRouter::next` / `ProviderRouter::mark_failed` (call site: `main.rs` fn `run_bot_turn`) | Rename `next` to `current` (and optionally `mark_failed` to `fail_over`) so the peek semantics stop reading like `Iterator::next`, updating the call site and the routing unit tests | n |

## WP-61 - bot (`Cargo.toml`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `bo F16` | `rust/bot/Cargo.toml` `[dependencies]` keys `time`, `brdgme_markup`, `tokio` | Drop the unused `time` dep; keep `brdgme_markup` only if `bo F13` adopts it (otherwise drop it); **leave the tokio `signal` feature alone** - WP-39 Task 6 gave it a real consumer | n |
| `dp F7` | `rust/bot/Cargo.toml` `[dependencies]` key `getrandom` | Delete the direct `getrandom = "0.3"` line once `bo F12` removes its only use, collapsing the bot-vs-web 0.3/0.4 split - **same edit as `bo F12` above** | n |

## WP-43 - web (`src/bin/import_game.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `ws F63` | `rust/web/src/bin/import_game.rs` fn `main` (the `fs::read_to_string` call) | Optional polish only (dev-only tool, trusted local input): add a size sanity check with a clear error before slurping the bundle, or close as won't-fix | n |

## WP-43 - web (`Cargo.toml`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `ws F64` | `rust/web/Cargo.toml` `[dependencies]` key `gloo-net` | Delete the `gloo-net` line entirely (zero `gloo_net` references in `src/` or `tests/`); **keep `gloo-timers`, which is used** | n |
| `ws F65` | `rust/web/Cargo.toml` `[dependencies]` key `tokio` | Add `"net"` and `"time"` to the tokio feature list - both are used directly (`tokio::net::TcpListener`, `tokio::time` in `websocket.rs` and `sweep.rs`) and only compile today via feature unification | n |
| `ws F66` | `rust/web/Cargo.toml` `[dependencies]` key `futures-util` + `[features] ssr` (+ `[[test]]` entries) | Make `futures-util` `optional = true` with `dep:futures-util` in `ssr`, **and** add it as a `[dev-dependencies]` entry (or give the affected integration tests `required-features = ["ssr"]`) or non-ssr `cargo test` breaks on the ungated top-level `use futures_util` in `tests/websocket_hygiene.rs` and `tests/nats_bot_eventing.rs` | y |
| `ws F67` | `rust/web/Cargo.toml` `[dependencies]` keys `async-nats`, `svix` | **UNVERIFIABLE (external basis)** - verification could confirm only the in-repo declared versions, not crates.io currency; re-check upstream, then bump `async-nats` 0.49 -> 0.50 (read the JetStream changelog) and `svix` 1.98 -> 1.99.1 if still applicable | n |

## Decision-blocked rows

None. Both WP-61 and WP-43 are READY and no row here is gated. Checked
`planning/decisions-needed.md`: the Group C dependency decisions all belong to
other packages - D-17 (sqlx 0.8/0.9, WP-66), D-18 (sentry feature trim,
WP-67), D-19 (`[workspace.dependencies]` migration, WP-64), D-21 (serde_yaml,
WP-70), D-23 (deny.toml, WP-69). None of them gates a row above:
`dp F7`/`bo F12` removes a direct dependency outright rather than restating a
version, so it does not wait on D-19. The parked rules-review packages
(WP-11, WP-12, WP-16, WP-20, WP-26, WP-30) own no findings in this batch.

## Not in this checklist (owned elsewhere)

- `bo F1`, `bo F3`, `bo F5` (reachable `unreachable!()`, unbounded spawn
  concurrency, no graceful shutdown) - owned by
  `specs/WP-39-bot-consumer-supervision.md`. WP-39's own scope note explicitly
  defers `bo F4`/`F6`/`F7`/`F9`/`F16` to WP-61, so the rows above do not
  overlap it; the one interaction is that WP-39 Task 6 now consumes the tokio
  `signal` feature, which is why `bo F16` must not remove it.
- `bo F2` (bot-turn wedge / ack-deadline) - owned by
  `specs/WP-38-bot-turn-wedge-recovery.md`.
- `bo F17` (`serde_yaml` 0.9 unmaintained) is **deliberately excluded**: it is
  WP-70, blocked on D-21, and cross-listed as `ls F34`/`dp F14` in
  `specs/WP-07-game-client-rand-bot.md`. Leave `serde_yaml = "0.9"` in
  `rust/bot/Cargo.toml` alone even though `bo F16` edits the same table.
- `bo F8` (`/healthz` probes NATS but not the DB) is outside WP-61's declared
  scope list and is not to be touched here.
- `dp F13` (term_size unmaintained) - owned by
  `specs/WP-68-term-size-replacement.md`. `bo F11`/`bo F12` touch
  `rust/bot/src/crypto.rs`, which `specs/WP-36-crypto-deploy-hardening.md`
  also reaches; WP-36 owns the key-management/deploy posture, these two rows
  own only the local API shape - land WP-36 first if both are in flight.
- `ws F30` (rejected review-wide) and every other `ws` finding outside
  F63-F67 belong to other web packages.

## Escalate

None. All 17 fixes compress to one line. `bo F9` is the widest (nine
`try_get` sites across two files) but the rule is uniform per site - nullable
column gets `Option<T>`, NOT NULL column gets `.context(...)?` - so it stays a
single Tier 3 row rather than an escalation.
