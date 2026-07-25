# Verification: lib/game_client + lib/rand_bot (F31-F37, F40-F45)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (f8763a5). All paths relative to that dir. Read-only verification; no code changed.

## F31 (major, correctness, lib/game_client/src/lib.rs:47) - CONFIRMED

Claim: retry doc/predicate assume timeouts, but crate sets no timeout; operator's client has none, so a hang blocks a reconcile worker forever.

Evidence:
- lib/game_client/src/lib.rs:12-15 doc: "Bounded retry policy for transient transport failures (connect-refused, timeouts)".
- lib/game_client/src/lib.rs:80: `let retryable = e.is_connect() || e.is_timeout();`
- The crate never calls `.timeout(...)` on the request or builds its own client; it uses the caller-supplied `&reqwest::Client` (lib.rs:47-61). Only test code sets a timeout (lib.rs:455-458).
- web/src/main.rs:32-36: `reqwest::Client::builder().connect_timeout(5s).timeout(10s)` - has timeout. (Caller claim correct.)
- bot/src/main.rs:786-790: `game_http = reqwest::Client::builder().connect_timeout(5s).timeout(60s)` - has timeout. (Caller claim correct; bot also builds a 300s `http` client at :781-785 for non-game calls.)
- operator/src/controller.rs:230: `let http = reqwest::Client::new();` - no timeout. Default reqwest clients have no total-request timeout.
- Reachability: operator/src/controller.rs:118 and :131 call `game_service_request(&ctx.http, ...)` from `reconcile`, which delegates to `brdgme_game_client::request` (controller.rs:47-56). A game pod that accepts the connection and never responds hangs `send()` indefinitely; `is_timeout()` never fires, and the awaiting reconcile task is stuck.

All three caller claims verified. Confirmed at major.

## F32 (minor, consistency, lib/game_client/src/lib.rs:102) - CONFIRMED

Evidence:
- lib/game_client/Cargo.toml:9: `anyhow = "1.0.103"`; lib.rs:7 `use anyhow::{Context, Result, anyhow};`.
- Non-2xx: lib.rs:102 `Err(anyhow!("game service returned {status}: {body}"))`; SystemError: lib.rs:107 `Err(anyhow!("{}", message))`; variant mismatch: lib.rs:170/190/234/254/268/279/289 `anyhow!("invalid response type")` etc.; transport failures propagate via `?` at lib.rs:98 into anyhow. All are string-typed, not machine-distinguishable.
- Other lib crates use thiserror: lib/cmd/Cargo.toml:13, lib/game/Cargo.toml:13, lib/markup/Cargo.toml:12, lib/color/Cargo.toml:12 (`thiserror = "2.0.18"`); no anyhow dep in those four Cargo.tomls (grep for `anyhow` returned nothing).

## F33 (minor, correctness, lib/game_client/src/lib.rs:80) - CONFIRMED

Evidence: lib.rs:80 `let retryable = e.is_connect() || e.is_timeout();`. In reqwest's error taxonomy, a connection accepted then reset mid-request/response surfaces as a request/body/decode error (`is_request()`/`is_body()`), not `is_connect()` (which covers connection establishment). Such errors hit the `!retryable` branch at lib.rs:81 and return immediately. This is exactly the transient class the doc comment (lib.rs:12) says the policy targets. Consistent with the crate's own tests, which only exercise connect-refused and timeout paths.

## F34 (minor, dependencies, lib/game_client/Cargo.toml:15) - CONFIRMED

Evidence:
- lib/game_client/Cargo.toml:15: `serde_yaml = "0.9"`.
- Workspace grep for serde_yaml in Cargo.toml files returns exactly two hits: bot/Cargo.toml:29 and lib/game_client/Cargo.toml:15 - bot is the only other user, as claimed.
- Deprecation status (dtolnay archived serde_yaml in 2024) taken as given per task instructions.

## F35 (nit, correctness, lib/game_client/src/lib.rs:54) - CONFIRMED

Evidence:
- lib.rs:54: `let host = format!("{version_name}.games.internal");` then lib.rs:60 `.header(reqwest::header::HOST, &host)` - no validation.
- Caller sources: bot/src/main.rs:89/117 pulls `gv.name as version_name` from DB and passes it (main.rs:223, 356, 382, 502); web passes DB-derived version names (e.g. web/src/proposals.rs:94 `version_name: String`); operator passes `obj.name_any()` (controller.rs:65, used as `name` in game_service_request at :118/:131) - k8s object name.
- reqwest's `.header()` with an invalid HeaderValue makes the builder error at send time (a reqwest builder error), so no header injection - but the failure is an opaque reqwest error, as described. Nit severity appropriate.

## F36 (nit, simplicity, lib/game_client/src/lib.rs:220) - CONFIRMED

Evidence: fetch_game_data issues sequential awaits: Status (lib.rs:220-226), then for interface_version >= 2 DataDocs (245-255), BasicStrategy (256-269), AdvancedStrategy (270-280), then Rules (287-290). The four post-Status requests do not depend on each other's results (only on `game`/`player` inputs; AdvancedStrategy consumes `game` by move but that is an ordering artifact, not a data dependency). Bot calls it per turn: bot/src/main.rs:499. Five round trips where two (Status, then join of four) would do.

## F37 (nit, quality, lib/game_client/src/lib.rs:322) - CONFIRMED

Evidence: test_retry_on_connect_refused_then_success (lib.rs:322-382) spawns the real server after a 15ms sleep (lib.rs:334) while the client's first backoff, with base_delay 40ms (lib.rs:356) and equal jitter (backoff_delay, lib.rs:39-45), lands in [20ms, 40ms]. If the spawned task is delayed past the first retry on a loaded runner, the second attempt also gets connect-refused; max_attempts=3 (lib.rs:359) leaves a third attempt at roughly +60-120ms cumulative, so outright failure is unlikely but the timing race is real. The elapsed-time assertion at lib.rs:377-381 uses a loosened 15ms bound, showing the authors already worked around timing sensitivity.

## F40 (minor, dependencies, lib/rand_bot/Cargo.toml:11) - CONFIRMED

Evidence:
- lib/rand_bot/Cargo.toml:11: `chrono = { version = "0.4.45", features = ["serde"] }`.
- `grep -rn chrono lib/rand_bot/` matches only the Cargo.toml line - no source usage in src/lib.rs or src/main.rs.
- lib/cmd/Cargo.toml:12: `time = { version = "0.3", ... }` - the time crate is what cmd uses, matching the standardization claim.

## F41 (minor, consistency, lib/rand_bot/src/lib.rs:93) - CONFIRMED

Evidence:
- lib/rand_bot/src/lib.rs:85: `command::Spec::Space => vec![" ".to_string()]` - explicit space token.
- lib/rand_bot/src/lib.rs:93: `commands()` does `.join(" ")` - a Space token surrounded by the join separator yields three spaces / double-spacing around Space tokens.
- tools/fuzz/src/lib.rs:349: `brdgme_rand_bot::spec_to_command(command_spec, command_spec, players, rng).join("")` - joins with empty string. Same generator, different output shape per driver, as claimed.

## F42 (minor, dependencies, lib/rand_bot/Cargo.toml:9) - CONFIRMED

Evidence:
- lib/rand_bot/Cargo.toml:9: `brdgme_cmd = { path = "../cmd" }` - no `default-features = false`.
- lib/cmd/Cargo.toml:24-25: `default = ["http-server"]`, `http-server = ["warp", "tokio", "sentry"]` (warp 0.4.3, tokio with signal, sentry 0.48 declared optional at lib/cmd/Cargo.toml:17-19).
- rand_bot is a stdin/stdout CLI (lib/rand_bot/src/main.rs:3-7), so it links warp/tokio/sentry needlessly. Contrast: lib/game_client/Cargo.toml:10 does use `default-features = false` on the same dep.

## F43 (minor, correctness, lib/rand_bot/src/lib.rs:50) - CONFIRMED

Evidence:
- lib.rs:49-51: `Spec::OneOf(ref options) => spec_to_command(options.choose(rng).unwrap(), ...)` - empty options panics on unwrap.
- lib.rs:84: `Spec::Player => vec![players.choose(rng).unwrap().to_owned()]` - empty players panics.
- lib.rs:107: `serde_json::from_reader::<_, bot_cli::Request>(input).unwrap()` - malformed request JSON panics; output writes at lib.rs:108-113 also unwrap.
- Contrast lib.rs:45-48: Enum uses `values.choose(rng).map(...).unwrap_or_else(Vec::new)` - graceful on empty.
- Recursion is structural over the spec (single pass; Many bounded via bounded_i32 at lib.rs:69-71), so no infinite-loop risk, as stated.

## F44 (nit, consistency, lib/rand_bot/src/main.rs:1) - CONFIRMED

Evidence: lib/rand_bot/src/main.rs:1: `extern crate brdgme_rand_bot;`. lib/rand_bot/Cargo.toml:6: `edition = "2024"`. Extern crate declarations for dependencies are unnecessary since edition 2018; the statement is dead weight.

## F45 (nit, quality, lib/rand_bot/src/lib.rs:98) - CONFIRMED

Evidence: lib.rs:98-101:

    // / Most bots just want to use `brdgme_cmd::bot_cli`, however because RandBot
    // doesn't care about game / state, we implement a more simplified version of
    // the CLI here. This allows the bot to be used / with arbitrary games as long
    // as the command spec is generated.

Mangled doc-comment wrap: `// /` instead of `///`, and stray `/` characters mid-sentence ("game / state", "used / with"). Repo-wide grep for `bot_cli` outside lib/cmd hits only rand_bot itself (lib.rs:5, :98, :107 - the type import, this comment, and the Request type); rand_bot is the only bot in the tree and it does not use a bot_cli-provided CLI, so the "most bots" claim describes usage no bot has.
