# Findings: bot-operator-tools

Scope: `bot/` (1,708 LOC, 6 files), `operator/` (412 LOC, 3 files),
`tools/fuzz` (358), `tools/render_plain` (32), `tools/repl` (10) - ~2,662
LOC. Snapshot `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. `lib/rand_bot`
was excluded (already reviewed in unit 2 lib-support). Raw worker dumps and
the review log are in `findings/raw/bot-operator-tools-*.md`; all major
findings were spot-checked by the Lead against the snapshot (loop shape at
main.rs:242/454, ack sites at main.rs:824-862, finalizer patches at
controller.rs:80-105, fuzz channel at tools/fuzz/src/lib.rs:23-59,
merge_json_patch recursion condition, crd printcolumn jsonPath).

The bot's ack-after-all-work pattern cross-references the same pattern
flagged in the web-server/web-domain units (NATS consumers with no
Progress/term handling); noted inline rather than re-scoped.

## bot/src/main.rs (turn loop, NATS consumer)

### `unreachable!()` is reachable - panic when final attempt takes a `continue` path
- severity: major
- category: correctness
- location: bot/src/main.rs:454
- finding: The retry loop is `for attempt in 0..MAX_ATTEMPTS { ... }` (main.rs:242) followed by `unreachable!()`. The exhaustion check at main.rs:420 (`if attempt + 1 == MAX_ATTEMPTS`) only runs on the command-validation-failure path. Two other paths `continue` without it: LLM call error (main.rs:311, after `router.mark_failed()`) and game-state-changed refresh (main.rs:372). If the final iteration (attempt 19) takes either `continue`, the loop exits normally and hits `unreachable!()`, panicking the spawned task. Concretely: 19 rejected commands, then on attempt 19 the game state changes mid-LLM-call -> refresh -> `continue` -> panic. The JoinHandle is dropped at main.rs:832, so the panic only surfaces via tokio's default printing; the message is left unacked and redelivered.
- recommendation: Replace the `for`/`unreachable!()` shape with a loop that checks the budget on every path (or move the exhaustion check to the top of the body) and return an `Err` instead of relying on `unreachable!()`.

### No ack-deadline extension for turns that can run for many minutes
- severity: major
- category: quality
- location: bot/src/main.rs:832-866
- finding: A bot turn can legitimately run for tens of minutes: each LLM call has a 300 s client timeout (main.rs:783) and the loop allows up to 20 attempts plus provider failover. The message is only acked after `run_bot_turn` completes (main.rs:856); there is no `AckKind::Progress` heartbeat while working. UNCERTAIN: the `bot-turn` consumer config (ack_wait, max_deliver) is owned by the monolith and not visible in this crate - but unless ack_wait exceeds the worst-case turn duration, JetStream will redeliver the message while the original task is still running, and the bot will process the same turn concurrently (`tokio::spawn` per message imposes no dedup). Both copies can pass the stateless `Play` validation against the same `game_state` and both publish `bot.command`, i.e. duplicate command submission. The DB re-check at main.rs:319-339 narrows but does not close this window. Matches the ack-after-all-work pattern flagged in the web units.
- recommendation: Send `ack_with(AckKind::Progress)` periodically from a heartbeat task while a turn is in flight, or verify/raise the consumer's ack_wait well above worst-case turn duration and bound the worst case.

### Unbounded concurrency: one `tokio::spawn` per message with no local limit
- severity: minor
- category: quality
- location: bot/src/main.rs:832
- finding: Every pulled message spawns a task with no semaphore or bound in this process; the only backpressure is the consumer's max_ack_pending, configured by the monolith and not visible here (UNCERTAIN on its value). A burst of turn events means that many concurrent LLM calls, DB queries, and game-service calls from one pod.
- recommendation: Bound in-process concurrency (e.g. `tokio::sync::Semaphore`) sized to what one pod should run, independent of the externally-owned consumer config.

### `merge_json_patch` deviates from the RFC 7396 behaviour its doc claims
- severity: minor
- category: correctness
- location: bot/src/main.rs:606-631
- finding: The doc comment says "Applies a JSON Merge Patch (RFC 7396)". RFC 7396 recurses whenever the patch value is an object, treating a non-object target as `{}` so nested nulls are stripped. Here the recursion condition is `patch_value.is_object() && target_entry.is_object()` (main.rs:625); when the target entry is absent (`or_insert(Null)`) or a scalar, the patch object is cloned verbatim, preserving nulls. Example: patch `{"thinking": {"budget": null, "type": "disabled"}}` against a body without `thinking` produces a literal `"budget": null` sent to the provider. The unit test at main.rs:900 only covers the nested case without nulls.
- recommendation: When `patch_value` is an object and the target entry is not, recurse against a fresh empty object (matching RFC 7396), or drop the RFC claim and document the actual semantics.

### No graceful shutdown - SIGTERM aborts in-flight turns mid-LLM-call
- severity: minor
- category: quality
- location: bot/src/main.rs:812-869
- finding: The consumer loop runs until the stream ends; there is no SIGTERM/ctrl-c handling, so a deploy or pod reschedule kills tasks that are minutes into an LLM call. The work is redelivered later (unacked) so no data is lost, but the LLM spend is wasted and the game stalls until redelivery. The enabled-but-unused tokio `signal` feature suggests this was intended.
- recommendation: On SIGTERM, stop pulling new messages and await in-flight tasks (track JoinHandles or use a TaskTracker) up to the pod's termination grace period.

### Self marker in the prompt keys on name equality, breaking with duplicate names
- severity: minor
- category: correctness
- location: bot/user_prompt.md:8 (built in bot/src/main.rs:558-572)
- finding: The player list marks "(you)" via `{% if player.name == my_name %}`. Player names come from `users.name`/`game_bots.name` with a positional fallback; nothing guarantees uniqueness within a game. If another player shares the bot's name, both entries are marked "(you)" and the LLM cannot tell which seat it holds. The Rust side already knows `player_position` but does not pass it to the template.
- recommendation: Pass `my_position` (or a per-player `is_me` flag computed from the index in `build_messages`) instead of comparing names.

### Trace log labelled "Rendered prompt" logs only the system message
- severity: nit
- category: quality
- location: bot/src/main.rs:269-274
- finding: `prompt = %messages.first()...` logs `messages[0]`, the static system prompt. The user message - the per-turn game state and command spec, which is what you need when debugging a bad bot move - is never logged.
- recommendation: Log both messages (or specifically the user message) at trace level.

### `/healthz` reports only NATS state, not DB
- severity: nit
- category: quality
- location: bot/src/main.rs:685-690
- finding: Health is `jetstream.client().connection_state() == Connected`; the Postgres pool is not probed, so a pod with a dead DB connection reports healthy while every turn fails.
- recommendation: Include a cheap pool check (e.g. `pool.acquire()` with a short timeout) if the deployment uses `/healthz` for readiness.

## bot/src/config.rs + crypto.rs

### Pervasive `try_get(...).unwrap_or(default)` silently masks decode/schema errors
- severity: minor
- category: quality
- location: bot/src/config.rs:35-38 (also config.rs:46, config.rs:71-74, main.rs:104, main.rs:118, main.rs:147-149, main.rs:330, main.rs:528)
- finding: Many column reads swallow errors into defaults, making schema drift or a type mismatch indistinguishable from legitimate data. Examples: `temperature: row.try_get("temperature").unwrap_or(0.2)` - a mis-typed column silently runs every bot at 0.2; `bots_table_empty` uses `unwrap_or(true)` for `has_bots`, so a decode error silently disables the env-fallback path and the bot skips turns; `load_bot_context` maps a failed `body` decode to an empty log line. Some uses are deliberate NULL handling from LEFT JOINs (e.g. `is_turn` at main.rs:104), but the pattern is applied indiscriminately to NOT NULL columns too.
- recommendation: Use `try_get::<Option<T>, _>` + `unwrap_or` where NULL is expected, and propagate errors (`.context(...)?`, as already done for `game_state`/`uri`) for columns that should always decode.

### `extra_body` env parse errors are swallowed
- severity: minor
- category: quality
- location: bot/src/config.rs:103-105
- finding: `std::env::var("LLM_EXTRA_BODY").ok().and_then(|raw| serde_json::from_str(&raw).ok())` - a typo in the env var's JSON silently yields `extra_body: None`, so the operator's override is dropped with no log. A test even enshrines this (`env_fallback_provider_invalid_extra_body_is_none`).
- recommendation: Log a warning (or fail startup) when `LLM_EXTRA_BODY` is set but unparseable; misconfiguration should be loud.

### `load_key` returns the insecure default key on missing env var
- severity: nit
- category: quality
- location: bot/src/crypto.rs:53-57
- finding: `load_key()` silently returns `default_key()` when `DATABASE_ENCRYPTION_KEY` is unset; the caller compensates with a separate `using_default_key()` check plus warning (main.rs:752). Two functions independently read the same env var to express one decision, and any future caller of `load_key` alone gets a production-insecure key with no signal.
- recommendation: Have `load_key` return an enum/flag (e.g. `Key::Default`/`Key::FromEnv`) and derive the warning from that single source.

### Bespoke nonce generation instead of aes-gcm's built-in
- severity: nit
- category: dependencies
- location: bot/src/crypto.rs:65-69
- finding: `rand_nonce()` fills 12 bytes via a direct `getrandom` dependency. `aes-gcm`'s `AeadCore::generate_nonce` (with the `aead/os_rng` feature) does this canonically, removing the hand-rolled helper and the extra top-level dep (which is also the getrandom 0.3-vs-0.4 drift crate; see the dependencies unit).
- recommendation: Use `Aes256Gcm::generate_nonce` and drop the direct `getrandom` dependency.

## bot/src/prompt.rs + routing.rs

### Hand-rolled `{{player N}}` substitution instead of the workspace markup library
- severity: minor
- category: consistency
- location: bot/src/prompt.rs:45-51
- finding: `markup_resolve_players` does raw `str::replace` of `{{player N}}` for each name, while `brdgme_markup` (a declared but unused dependency of this crate) provides parsing and a `transform` pass with `Player` nodes that the rest of the project uses for exactly this. The string approach also has an injection quirk: names are substituted sequentially, so a player name containing a literal `{{player 1}}` gets re-substituted by a later iteration.
- recommendation: Use `brdgme_markup::from_string` + `transform`, or at minimum document why raw replacement is preferred and drop the unused dependency.

### `ProviderRouter::next` is a peek, contradicting its name
- severity: minor
- category: simplicity
- location: bot/src/routing.rs:17-23
- finding: `next(&mut self)` takes `&mut self` and is named like `Iterator::next` but does not advance - it returns `providers.get(self.index)`; only `mark_failed` advances. At the call site (main.rs:243) `router.next()` inside a loop reads as consuming an element per iteration, which is exactly wrong: the same provider is intentionally reused across validation retries. The struct is a `Vec` plus an index with a misleading API, and `priority` is only used as a SQL sort key.
- recommendation: Rename to `current()` (and `mark_failed` to `advance()`/`fail_over()`), or replace with `slice::Iter` + `Peekable`.

### minijinja Environment rebuilt and templates recompiled per render
- severity: nit
- category: quality
- location: bot/src/prompt.rs:65-68,79-82
- finding: `render_system`/`render_user` construct a fresh `Environment` and re-parse the embedded template on every call - up to 20+ times per turn inside the retry loop. Cost is trivial next to the LLM call, but a `static LazyLock<Environment>` is the idiomatic minijinja pattern and surfaces template syntax errors once at startup.
- recommendation: Build the `Environment` once in a `LazyLock` and reuse it.

## bot/Cargo.toml

### Unused dependencies and features
- severity: minor
- category: dependencies
- location: bot/Cargo.toml:12,27,30
- finding: `time = { version = "0.3", features = ["serde"] }` is never used in src/ (only `std::time`/`tokio::time` appear). `brdgme_markup = { path = "../lib/markup" }` is declared but never imported (prompt.rs hand-rolls player-tag substitution instead). tokio's `"signal"` feature is enabled but no signal handling exists.
- recommendation: Remove `time` and `brdgme_markup` (or actually use the latter), and drop the `signal` feature or implement the graceful shutdown it implies.

### serde_yaml 0.9 is unmaintained/archived
- severity: minor
- category: dependencies
- location: bot/Cargo.toml:29
- finding: `serde_yaml = "0.9"` was deprecated and archived by its maintainer in early 2024 and receives no fixes. It is used only in `spec_to_yaml` (prompt.rs:60-63) for output-only serialisation, so risk is low, but the review criteria call for maintained deps.
- recommendation: Move to a maintained fork (e.g. `serde_yaml_ng`/`serde-yml`) or emit the spec as JSON, which the LLM handles equally well.

## operator/

### Hand-rolled finalizer handling instead of kube-rs finalizer helper; merge patch can clobber concurrent finalizer edits
- severity: major
- category: consistency
- location: operator/src/controller.rs:80-105
- finding: Finalizer add/remove is implemented by hand: the reconciler builds the full finalizer list from the (watch-cache, possibly stale) object and writes it back with `Patch::Merge(json!({ "metadata": { "finalizers": finalizers } }))`. A merge patch replaces the entire array, so if any other actor (another controller, `foregroundDeletion`, a user) added a finalizer between the watch event and the patch, that finalizer is silently removed - and conversely a finalizer removed elsewhere can be resurrected. kube-rs ships `kube::runtime::finalizer::finalizer()` exactly for this: it uses JSON patches with a `test` op on the finalizer index plus resourceVersion semantics, and cleanly splits apply/cleanup paths. This is both a correctness race and a departure from the framework idiom.
- recommendation: Replace the manual deletion-timestamp/finalizer logic in `reconcile` with `kube::runtime::finalizer::finalizer(&api, FINALIZER, obj, |event| ...)` dispatching to apply/cleanup closures.

### CRD printcolumn points at a spec field that does not exist
- severity: minor
- category: correctness
- location: operator/src/crd.rs:18
- finding: `printcolumn = r#"{"name":"Players","type":"string","jsonPath":".spec.playerCounts"}"#` - `GameVersionSpec` has no `playerCounts` field (player counts are fetched from the game service at reconcile time, controller.rs:117-129, and stored only in Postgres). The `kubectl get gameversions` "Players" column will always render empty.
- recommendation: Drop the printcolumn, or surface player counts in `GameVersionStatus` and point the jsonPath at `.status.playerCounts`.

### Status message field is dead; errors never surface in CRD status
- severity: minor
- category: quality
- location: operator/src/crd.rs:43 and operator/src/controller.rs:155-160,223-226
- finding: `GameVersionStatus.message` is never written anywhere: the success path patches only `{ "ready": true, "observedGeneration": ... }` and `error_policy` only logs and requeues. A GameVersion whose game service is unreachable sits with no status at all (or a stale `ready: true` from a previous generation); the operator's diagnosis is only visible in pod logs.
- recommendation: In `error_policy` (or a wrapper around `reconcile`) patch status to `{ "ready": false, "message": err.to_string(), "observedGeneration": generation }`; clear `message` on success.

### Status patched via ad-hoc json! instead of the typed GameVersionStatus
- severity: minor
- category: consistency
- location: operator/src/controller.rs:155-160
- finding: The crate defines `GameVersionStatus` (crd.rs:38-46) but the reconciler patches status with a hand-built `json!({ "status": { "ready": true, "observedGeneration": generation } })`. Field-name drift between the struct's serde renames and the literal JSON keys is only caught by eyeball, not the compiler.
- recommendation: Build a `GameVersionStatus { ready: true, message: None, observed_generation: generation }` and serialize it into the patch.

### weight bound as f64 against a real (float4) column
- severity: minor
- category: quality
- location: operator/src/controller.rs:193
- finding: `.bind(weight as f64)` widens the `f32` spec value to `f64` and sends it as FLOAT8, relying on Postgres's implicit float8 -> float4 assignment cast into `game_types.weight real` (web/migrations/001_initial_schema.sql:146). sqlx encodes `f32` as FLOAT4 natively, and the test at controller.rs:275 reads the column back as `f32` anyway.
- recommendation: `.bind(weight)` and drop the cast.

### Redundant serde rename on interface_version
- severity: nit
- category: simplicity
- location: operator/src/crd.rs:34
- finding: `#[serde(rename = "interfaceVersion", default = "default_interface_version")]` - the struct already has `#[serde(rename_all = "camelCase")]` (crd.rs:10), which produces `interfaceVersion`; the explicit rename is redundant.
- recommendation: Keep only `#[serde(default = "default_interface_version")]`.

### interceptor_uri test depends on ambient environment
- severity: nit
- category: quality
- location: operator/src/controller.rs:247-254
- finding: `interceptor_uri_defaults_to_keda_proxy` asserts the default value and relies on `INTERCEPTOR_URI` being unset in the test environment (acknowledged in its comment). Any developer or CI environment that exports the var makes an unrelated test fail.
- recommendation: Remove the test (it asserts a string literal against itself) or refactor `interceptor_uri` to take an `Option<String>` and test the fallback purely.

### k8s-openapi "latest" feature instead of pinning the cluster's API version
- severity: nit
- category: dependencies
- location: operator/Cargo.toml:14
- finding: `k8s-openapi = { version = "0.28", features = ["latest"] }`. k8s-openapi's guidance is that binaries should enable the version feature matching the oldest cluster they target; `latest` silently moves the compiled-against API surface on every upgrade. For this operator (only CRD types plus client machinery) the practical risk is low. UNCERTAIN: acceptable if the cluster is kept current with the workspace.
- recommendation: Pin the versioned feature (e.g. `v1_32`) matching the deployed cluster, or document why `latest` is intended.

## tools/fuzz

### Worker-thread failure hangs the fuzzer forever (main keeps a live Sender)
- severity: major
- category: correctness
- location: tools/fuzz/src/lib.rs:23-59
- finding: `let (step_tx, step_rx) = channel();` - the original `step_tx` stays alive in `fuzz()` for its whole duration; only clones are moved into worker threads (lib.rs:27). If every worker dies (e.g. `Fuzzer::try_new(...).expect(...)` at lib.rs:32 panics because the requester binary path is wrong, or `requester::parse_args(&args).unwrap()` in tools/fuzz/src/main.rs:7 panics inside the closure), all clones are dropped but the channel never disconnects, so `step_rx.recv().expect("failed to get step")` at lib.rs:59 blocks forever. The user sees per-thread panic messages then a silent hang instead of an exit.
- recommendation: `drop(step_tx);` immediately after the spawn loop so `recv()` returns `Err` once all workers are gone, and treat that `Err` as "all workers exited" instead of `expect`ing.

### SystemTime used for interval timing; can panic if the clock steps backwards
- severity: minor
- category: quality
- location: tools/fuzz/src/lib.rs:46-53
- finding: `last_output_at` is a `SystemTime` and the loop does `now.duration_since(last_output_at).expect("failed to get duration")`. `SystemTime` is not monotonic - an NTP step or suspend/resume adjustment makes `duration_since` return `Err` and the fuzzer panics mid-run.
- recommendation: Use `Instant::now()` / `Instant::elapsed`, which cannot fail.

### num_cpus dependency duplicates std's available_parallelism
- severity: minor
- category: dependencies
- location: tools/fuzz/Cargo.toml:13 (used at tools/fuzz/src/lib.rs:25)
- finding: `num_cpus = "1.17.0"` is pulled in solely for `num_cpus::get()`. Since Rust 1.59, `std::thread::available_parallelism()` covers this (both respect cgroup CPU quotas, so behaviour is equivalent for this use).
- recommendation: `std::thread::available_parallelism().map(NonZero::get).unwrap_or(1)` and drop the dependency.

### Factory wrapped in Arc<Mutex<>> only to smuggle a Fn across threads
- severity: nit
- category: simplicity
- location: tools/fuzz/src/lib.rs:22,31
- finding: `let new_requester = Arc::new(Mutex::new(new_requester));` then `new_requester.lock().unwrap()()` exists only because the bound is `F: Fn() -> R + Send` without `Sync`. Both call sites (the `parse_args` closure in main.rs and `requester::gamer::new::<G>`) satisfy `Sync`.
- recommendation: Require `Sync`, share via `Arc<F>`, and call directly.

### Exit-signal send unwraps can panic when a worker died early
- severity: nit
- category: quality
- location: tools/fuzz/src/lib.rs:87-89
- finding: `tx.send(()).unwrap()` at shutdown panics if the corresponding worker thread already exited (its `exit_rx` dropped), e.g. after a worker panicked in `try_new`. The shutdown notification is best-effort by nature.
- recommendation: `let _ = tx.send(());`.

### Whole PlayerRender cloned to extract one field
- severity: nit
- category: simplicity
- location: tools/fuzz/src/lib.rs:201
- finding: `(player, player_render.clone().command_spec.unwrap(), state)` clones the entire `PlayerRender` (including render strings) to take `command_spec`, immediately after a separate `is_none()` check at lib.rs:198.
- recommendation: `let Some(spec) = player_render.command_spec.clone() else { return Err(...) };` and use `spec`.

## Areas reviewed and found clean

- bot/src/main.rs: LEFT JOIN NULL handling for absent players; pre- and post-LLM `is_turn`/`game_state` rechecks; stateless `Play` validation before publishing `bot.command`; failed-command feedback accumulation and reset on context refresh; JetStream publish with double-await ack; unparseable-payload messages acked to avoid poison loops; hard failures deliberately left unacked for redelivery (comment matches behaviour); sentry transaction continuation from NATS headers; two reqwest clients with distinct, commented timeouts; healthz wiring and startup ordering.
- bot/src/config.rs: SQL for bot/provider loading (enabled filters, priority ordering); decryption error propagation; env-fallback gating; env tests serialised behind a Mutex.
- bot/src/crypto.rs: AES-256-GCM with random 96-bit nonce prepended, length check before split, no nonce reuse; tamper/wrong-key/round-trip/hex-parse tests present and correct.
- bot/src/nats.rs: constants and event structs mirror web's definitions (field names/types match web's BotTurnEvent).
- bot/src/routing.rs: sequential failover behaviour correct and fully unit-tested.
- bot/src/prompt.rs: rendering contexts match template variables exactly (verified against both templates incl. `{% raw %}` guards); thorough template test coverage.
- bot Cargo.toml (remaining deps), system_prompt.md/user_prompt.md, .mirrord config: checked and CLEAN.
- operator/src/main.rs: crypto-provider install before kube client creation matches the documented workspace constraint; health endpoint documented and tested; startup ordering sound.
- operator/src/controller.rs (beyond findings): deletion-path DB update idempotent and correctly scoped; observed-generation short-circuit correct; requeue jitter hack explicitly justified; upserts match unique constraints (verified against web/migrations); good `#[sqlx::test]` coverage of the upsert path.
- operator/src/crd.rs schema derives, defaults, camelCase serialization; operator Cargo.toml (rustls provider comment accurate; sqlx 0.9/reqwest 0.13.4 confirmed via Cargo.lock); mirrord config: CLEAN apart from findings above.
- tools/fuzz (beyond findings): fuzz stepping, state threading (stateless server, state passed per request), seed + command-log repro reporting, remaining-input handling all correct.
- tools/render_plain: CLEAN in full (cross-language markup contract respected per charter; `expect` panics appropriate for a dev tool; player color indexing matches lib/color).
- tools/repl: CLEAN in full (minimal wrapper over lib/cmd's repl).

## Tally

- bot: 0 critical, 2 major, 9 minor, 5 nit (16)
- operator: 0 critical, 1 major, 4 minor, 3 nit (8)
- tools (fuzz/render_plain/repl): 0 critical, 1 major, 2 minor, 3 nit (6)
- Unit total: 0 critical, 4 major, 15 minor, 11 nit (30 findings)
