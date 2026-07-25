# Raw findings: bot crate (Unit 12 W1)

Scope: rust/bot at snapshot f8763a5 - src/main.rs (920), src/config.rs (189), src/crypto.rs (140), src/nats.rs (35), src/prompt.rs (356), src/routing.rs (68), Cargo.toml, system_prompt.md, user_prompt.md, .mirrord/mirrord.json. All files read in full. lib/rand_bot excluded per instructions.

### `unreachable!()` is reachable - panic when final attempt takes a `continue` path
- severity: major
- category: correctness
- location: bot/src/main.rs:454
- finding: The retry loop is `for attempt in 0..MAX_ATTEMPTS { ... }` followed by `unreachable!()`. The `MAX_ATTEMPTS` exhaustion check at line 420 (`if attempt + 1 == MAX_ATTEMPTS`) only runs on the command-validation-failure path. Two other paths `continue` without that check: LLM call error (line 311, after `router.mark_failed()`) and game-state-changed refresh (line 372). If the final iteration (attempt 19) takes either `continue`, the loop exits normally and falls through to `unreachable!()`, panicking the spawned task. Concretely: 19 rejected commands, then on attempt 19 the game state changes mid-LLM-call -> refresh -> `continue` -> panic. The panic is only surfaced via tokio's default panic printing (the JoinHandle is dropped at main.rs:832), the message is left unacked and redelivered - but this is a genuine latent panic, not control flow.
- recommendation: Replace the `for`/`unreachable!()` shape with a loop that decrements/checks a budget on every path, or move the exhaustion check to the top of the loop body; return an `Err` instead of relying on `unreachable!()`.

### No ack-deadline extension for turns that can run for many minutes
- severity: major
- category: quality
- location: bot/src/main.rs:832-866
- finding: A bot turn can legitimately run for a very long time: each LLM call has a 300 s client timeout (main.rs:783) and the loop allows up to 20 attempts plus provider failover, so a single message can be in flight for tens of minutes. The message is only acked after `run_bot_turn` completes (line 856); there is no `AckKind::Progress` heartbeat while working. UNCERTAIN: the `bot-turn` consumer config (ack_wait, max_deliver) is owned by the monolith and not visible in this crate - but unless ack_wait exceeds the worst-case turn duration, JetStream will redeliver the message while the original task is still running, and the bot will process the same turn concurrently (the `tokio::spawn` per message imposes no dedup). Both copies can pass the stateless `Play` validation against the same `game_state` and both publish `bot.command`, i.e. duplicate command submission. The DB re-check at lines 319-339 narrows but does not close this window. This matches the ack-after-all-work pattern already flagged in the web unit.
- recommendation: Send `ack_with(AckKind::Progress)` periodically (e.g. from a heartbeat task) while a turn is in flight, or verify/raise the consumer's ack_wait well above worst-case turn duration and bound the worst case (see MAX_ATTEMPTS x 300 s).

### Unbounded concurrency: one `tokio::spawn` per message with no local limit
- severity: minor
- category: quality
- location: bot/src/main.rs:832
- finding: Every pulled message spawns a task with no semaphore or bound in this process; the only backpressure is the consumer's max_ack_pending, which is configured by the monolith and not visible here (UNCERTAIN on its value). A burst of turn events means that many concurrent LLM calls, DB queries, and game-service calls from one pod.
- recommendation: Bound in-process concurrency (e.g. `tokio::sync::Semaphore`) sized to what one pod should run, independent of the externally-owned consumer config.

### `merge_json_patch` deviates from the RFC 7396 behaviour its doc claims
- severity: minor
- category: correctness
- location: bot/src/main.rs:606-631
- finding: The doc comment says "Applies a JSON Merge Patch (RFC 7396)". RFC 7396 recurses whenever the patch value is an object, treating a non-object target as `{}` so nested nulls are stripped. Here the recursion condition is `patch_value.is_object() && target_entry.is_object()` (line 625); when the target entry is absent (`or_insert(Null)`) or a scalar, the patch object is cloned verbatim, preserving nulls. Example: patch `{"thinking": {"budget": null, "type": "disabled"}}` against a body without `thinking` produces `"thinking": {"budget": null, "type": "disabled"}` instead of `{"type": "disabled"}` - a literal `null` sent to the provider. The unit test at line 900 only covers the case where `thinking`'s patch object contains no nulls.
- recommendation: When `patch_value` is an object and the target entry is not, recurse against a fresh empty object (matching RFC 7396), or drop the RFC claim and document the actual semantics.

### Pervasive `try_get(...).unwrap_or(default)` silently masks decode/schema errors
- severity: minor
- category: quality
- location: bot/src/config.rs:35-38 (also config.rs:46, config.rs:71-74, main.rs:104, main.rs:118, main.rs:147-149, main.rs:330, main.rs:528)
- finding: Many column reads swallow errors into defaults, making a schema drift or type mismatch indistinguishable from legitimate data. Examples: `temperature: row.try_get("temperature").unwrap_or(0.2)` - a mis-typed column silently runs every bot at 0.2; `bots_table_empty` uses `unwrap_or(true)` for `has_bots`, so a decode error silently disables the env-fallback/default-config path and the bot skips turns; `load_bot_context` maps a failed `body` decode to an empty log line. Some uses are deliberate NULL handling from LEFT JOINs (e.g. `is_turn` at main.rs:104), but the pattern is applied indiscriminately to NOT NULL columns too.
- recommendation: Use `try_get::<Option<T>, _>` + `unwrap_or` where NULL is the expected case, and propagate errors (`.context(...)?`, as already done for `game_state`/`uri`) for columns that should always decode.

### Unused dependencies and features in Cargo.toml
- severity: minor
- category: dependencies
- location: bot/Cargo.toml:12,27,30
- finding: `time = { version = "0.3", features = ["serde"] }` is never used in src/ (only `std::time`/`tokio::time` appear). `brdgme_markup = { path = "../lib/markup" }` is declared but never imported - prompt.rs hand-rolls player-tag substitution instead (see separate finding). tokio's `"signal"` feature is enabled but no signal handling exists in main.rs.
- recommendation: Remove `time` and `brdgme_markup` (or actually use the latter), and drop the `signal` feature or implement the graceful shutdown it implies.

### serde_yaml 0.9 is unmaintained/archived
- severity: minor
- category: dependencies
- location: bot/Cargo.toml:29
- finding: `serde_yaml = "0.9"` - the crate was deprecated and archived by its maintainer in early 2024 and receives no fixes. It is used only in `spec_to_yaml` (prompt.rs:60-63) for output-only serialisation, so risk is low, but the review criteria call for maintained deps.
- recommendation: Move to a maintained fork/alternative (e.g. `serde_yaml_ng`/`serde-yml`, whichever the workspace standardises on) or emit the spec as JSON, which the LLM handles equally well.

### No graceful shutdown - SIGTERM aborts in-flight turns mid-LLM-call
- severity: minor
- category: quality
- location: bot/src/main.rs:812-869
- finding: The consumer loop runs until the stream ends; there is no SIGTERM/ctrl-c handling, so a deploy or pod reschedule kills tasks that are minutes into an LLM call. The work is redelivered later (unacked), so no data is lost, but the LLM spend is wasted and the game stalls until redelivery. The enabled-but-unused tokio `signal` feature suggests this was intended.
- recommendation: On SIGTERM, stop pulling new messages and await in-flight tasks (track JoinHandles or use a TaskTracker) up to the pod's termination grace period.

### `ProviderRouter::next` is a peek, contradicting its name; struct is an index wrapper
- severity: minor
- category: simplicity
- location: bot/src/routing.rs:17-23
- finding: `next(&mut self)` takes `&mut self` and is named like `Iterator::next` but does not advance - it returns `providers.get(self.index)`; only `mark_failed` advances. At the call site (main.rs:243) `router.next()` inside a loop reads as consuming an element per iteration, which is exactly wrong: the same provider is reused across validation retries by design. The whole struct is a `Vec` plus an index with a misleading API, and `priority` is only used as a SQL sort key (routing itself never reads it).
- recommendation: Rename to `current()` (and `mark_failed` to `advance()`/`fail_over()`), or replace the struct with a plain `slice::Iter` + `Peekable` if the peek semantics are not needed.

### Nested-object null handling aside, `extra_body` env parse errors are swallowed
- severity: minor
- category: quality
- location: bot/src/config.rs:103-105
- finding: `std::env::var("LLM_EXTRA_BODY").ok().and_then(|raw| serde_json::from_str(&raw).ok())` - a typo in the env var's JSON silently yields `extra_body: None`, so the operator's override is dropped with no log. There is even a test enshrining this (`env_fallback_provider_invalid_extra_body_is_none`).
- recommendation: Log a warning (or fail startup) when `LLM_EXTRA_BODY` is set but unparseable; misconfiguration should be loud.

### Self marker in the prompt keys on name equality, breaking with duplicate names
- severity: minor
- category: correctness
- location: bot/user_prompt.md:8 (built in bot/src/main.rs:558-572)
- finding: The player list marks "(you)" via `{% if player.name == my_name %}`. Player names come from `users.name`/`game_bots.name` with a positional fallback; nothing here guarantees uniqueness within a game. If another player shares the bot's name, both entries are marked "(you)" and the LLM cannot tell which seat it holds. The Rust side already knows `player_position` but does not pass it to the template.
- recommendation: Pass `my_position` (or a per-player `is_me` flag computed from the index in `build_messages`) instead of comparing names.

### Hand-rolled `{{player N}}` substitution instead of the workspace markup library
- severity: minor
- category: consistency
- location: bot/src/prompt.rs:45-51
- finding: `markup_resolve_players` does raw `str::replace` of `{{player N}}` for each name, while `brdgme_markup` (already a declared dependency of this crate, unused) provides parsing and a `transform` pass with `Player` nodes that the rest of the project uses for exactly this. The string approach also has an injection quirk: names are substituted sequentially, so a player name containing a literal `{{player 1}}` gets re-substituted by a later iteration.
- recommendation: Use `brdgme_markup::from_string` + `transform` (rendering with `plain`/keeping markup as needed), or at minimum document why raw replacement is preferred and drop the unused dependency.

### `load_key` returns the insecure default key on missing env var
- severity: nit
- category: quality
- location: bot/src/crypto.rs:53-57
- finding: `load_key()` silently returns `default_key()` when `DATABASE_ENCRYPTION_KEY` is unset; the caller compensates with a separate `using_default_key()` check plus warning (main.rs:752). Two functions independently read the same env var to express one decision, and any future caller of `load_key` alone gets a production-insecure key with no signal.
- recommendation: Have `load_key` return an enum/flag (e.g. `Key::Default`/`Key::FromEnv`) or `Option`, and derive the warning from that single source.

### Bespoke nonce generation instead of aes-gcm's built-in
- severity: nit
- category: dependencies
- location: bot/src/crypto.rs:65-69
- finding: `rand_nonce()` fills 12 bytes via a direct `getrandom` dependency. `aes-gcm`'s `AeadCore::generate_nonce` with the `aead/os_rng` feature does this canonically, removing the hand-rolled helper and the extra top-level dep.
- recommendation: Use `Aes256Gcm::generate_nonce` and drop the direct `getrandom` dependency.

### `/healthz` reports only NATS state, not DB
- severity: nit
- category: quality
- location: bot/src/main.rs:685-690
- finding: Health is `jetstream.client().connection_state() == Connected`; the Postgres pool is not probed, so a pod with a dead DB connection reports healthy while every turn fails.
- recommendation: Include a cheap pool check (`pool.acquire()` with a short timeout or sqlx's `Pool::is_closed` at minimum) if the deployment uses `/healthz` for readiness.

### Trace log labelled "Rendered prompt" logs only the system message
- severity: nit
- category: quality
- location: bot/src/main.rs:269-274
- finding: `prompt = %messages.first()...` logs `messages[0]`, the system prompt (static rules/strategy). The user message - the per-turn game state and command spec, which is what you need when debugging a bad bot move - is never logged.
- recommendation: Log both messages (or specifically the user message) at trace level.

### minijinja Environment rebuilt and templates recompiled per render
- severity: nit
- category: quality
- location: bot/src/prompt.rs:65-68,79-82
- finding: `render_system`/`render_user` construct a fresh `Environment` and re-parse the embedded template on every call - up to 20+ times per turn inside the retry loop. Cost is trivial next to the LLM call, but a `static LazyLock<Environment>` is the idiomatic minijinja pattern and also surfaces template syntax errors once at startup.
- recommendation: Build the `Environment` once in a `LazyLock` and reuse it.

## Areas reviewed and found clean

- bot/src/main.rs: DB row NULL handling via LEFT JOINs for absent players (is_turn gate before requiring `game_player_id`); pre- and post-LLM `is_turn`/`game_state` rechecks; stateless `Play` validation before publishing `bot.command`; failed-command feedback accumulation and reset on context refresh; JetStream publish with double-await ack; unparseable-payload messages acked to avoid poison loops; hard failures deliberately left unacked for redelivery (comment matches behaviour); sentry transaction continuation from NATS headers; two reqwest clients with distinct, commented timeouts; `merge_json_patch` top-level semantics and tests (nested-null case excepted, flagged above); healthz wiring and startup ordering (wait_for_turn_consumer backoff for monolith-owned consumer).
- bot/src/config.rs: SQL for bot/provider loading (enabled filters on all three tables, priority ordering); decryption error propagation (does not fall back on bad ciphertext); env-fallback gating on empty bots table; env tests correctly serialised behind a Mutex with unsafe set_var acknowledged.
- bot/src/crypto.rs: AES-256-GCM with random 96-bit nonce prepended to ciphertext, length check before split, no nonce reuse; tamper/wrong-key/round-trip/hex-parse tests all present and correct.
- bot/src/nats.rs: constants and event structs mirror web's definitions (field names/types match the BotTurnEvent published by web); doc comment states ownership of stream/consumer creation.
- bot/src/routing.rs: sequential failover behaviour is correct and fully unit-tested (naming flagged above).
- bot/src/prompt.rs: template rendering contexts match template variables exactly (verified against system_prompt.md and user_prompt.md, including `{% raw %}` guards around markup examples); `spec_to_yaml` JSON-detour rationale is documented and test-asserted; thorough template test coverage.
- bot/Cargo.toml: remaining deps (tokio, serde, reqwest+rustls, sqlx, axum 0.8, async-nats, minijinja 2, anyhow, tracing, sentry, aes-gcm, thiserror, hex) are mainstream and current; unused entries flagged above.
- system_prompt.md / user_prompt.md: single-command contract, parser-rule docs, and time-limit language consistent with the code's retry design.
- .mirrord/mirrord.json: dev tooling config only, no concerns.

## Tally

critical: 0, major: 2, minor: 9, nit: 5 (16 findings total)
