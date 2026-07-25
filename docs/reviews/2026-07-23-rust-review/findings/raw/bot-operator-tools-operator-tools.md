# Raw findings: operator + tools (Unit 12 W2)

Scope: full read of `operator/` (main.rs, controller.rs, crd.rs, Cargo.toml, .mirrord/mirrord.json), `tools/fuzz/` (main.rs, lib.rs, Cargo.toml), `tools/render_plain/` (main.rs, Cargo.toml), `tools/repl/` (main.rs, Cargo.toml) at snapshot commit f8763a5. Cross-checked against lib/game_client, lib/cmd requester API, lib/rand_bot API, web/migrations schema, and workspace Cargo.lock. lib/rand_bot and lib/cmd themselves not reviewed (out of scope).

### Hand-rolled finalizer handling instead of kube-rs finalizer helper; merge patch can clobber concurrent finalizer edits
- severity: major
- category: consistency
- location: operator/src/controller.rs:80-105
- finding: Finalizer add/remove is implemented by hand: the reconciler builds the full finalizer list from the (watch-cache, possibly stale) object and writes it back with `Patch::Merge(json!({ "metadata": { "finalizers": finalizers } }))`. A merge patch replaces the entire array, so if any other actor (another controller, `foregroundDeletion`, a user) added a finalizer between the watch event and the patch, that finalizer is silently removed - and conversely a finalizer removed elsewhere can be resurrected. kube-rs ships `kube::runtime::finalizer::finalizer()` exactly for this: it uses JSON patches with a `test` op on the finalizer index plus resourceVersion semantics, and cleanly splits apply/cleanup paths. Hand-rolling this is both a correctness race and a departure from the framework idiom.
- recommendation: Replace the manual deletion-timestamp/finalizer logic in `reconcile` with `kube::runtime::finalizer::finalizer(&api, FINALIZER, obj, |event| ...)` dispatching to apply/cleanup closures.

### Worker-thread failure hangs the fuzzer forever (main keeps a live Sender)
- severity: major
- category: correctness
- location: tools/fuzz/src/lib.rs:23-59
- finding: `let (step_tx, step_rx) = channel();` - the original `step_tx` stays alive in `fuzz()` for its whole duration; only clones are moved into worker threads (line 27). If every worker dies (e.g. `Fuzzer::try_new(...).expect(...)` at line 32 panics because the requester binary path is wrong, or `requester::parse_args(&args).unwrap()` in `tools/fuzz/src/main.rs:7` panics inside the closure), all clones are dropped but the channel never disconnects, so `step_rx.recv().expect("failed to get step")` at line 59 blocks forever. The user sees per-thread panic messages then a silent hang instead of an exit.
- recommendation: `drop(step_tx);` immediately after the spawn loop so `recv()` returns `Err` once all workers are gone, and treat that `Err` as "all workers exited" instead of `expect`ing.

### CRD printcolumn points at a spec field that does not exist
- severity: minor
- category: correctness
- location: operator/src/crd.rs:18
- finding: `printcolumn = r#"{"name":"Players","type":"string","jsonPath":".spec.playerCounts"}"#` - `GameVersionSpec` has no `playerCounts` field (player counts are fetched from the game service at reconcile time, controller.rs:117-129, and stored only in Postgres). The `kubectl get gameversions` "Players" column will always render empty.
- recommendation: Drop the printcolumn, or surface player counts in `GameVersionStatus` and point the jsonPath at `.status.playerCounts`.

### SystemTime used for interval timing; can panic if the clock steps backwards
- severity: minor
- category: quality
- location: tools/fuzz/src/lib.rs:46-53
- finding: `last_output_at` is a `SystemTime` and the loop does `now.duration_since(last_output_at).expect("failed to get duration")`. `SystemTime` is not monotonic - an NTP step or suspend/resume clock adjustment makes `duration_since` return `Err` and the fuzzer panics mid-run. `std::time::Instant` is the correct type for measuring elapsed intervals.
- recommendation: Use `Instant::now()` / `Instant::elapsed`, which cannot fail.

### num_cpus dependency duplicates std's available_parallelism
- severity: minor
- category: dependencies
- location: tools/fuzz/Cargo.toml:13 (used at tools/fuzz/src/lib.rs:25)
- finding: `num_cpus = "1.17.0"` is pulled in solely for `num_cpus::get()`. Since Rust 1.59, `std::thread::available_parallelism()` covers this (and additionally respects cgroup CPU quotas, which `num_cpus::get()` also does, so behaviour is equivalent for this use).
- recommendation: `std::thread::available_parallelism().map(NonZero::get).unwrap_or(1)` and drop the dependency.

### Status message field is dead; errors never surface in CRD status
- severity: minor
- category: quality
- location: operator/src/crd.rs:43 and operator/src/controller.rs:155-160,223-226
- finding: `GameVersionStatus.message` is never written anywhere: the success path patches only `{ "ready": true, "observedGeneration": ... }` and `error_policy` only logs and requeues. A GameVersion whose game service is unreachable sits with no status at all (or a stale `ready: true` from a previous generation) and the operator's diagnosis is only visible in pod logs. The field exists precisely for this and is dead weight as written.
- recommendation: In `error_policy` (or a wrapper around `reconcile`) patch status to `{ "ready": false, "message": err.to_string(), "observedGeneration": generation }`; clear `message` on success (merge patch with `"message": null` or use a typed status).

### Status patched via ad-hoc json! instead of the typed GameVersionStatus
- severity: minor
- category: consistency
- location: operator/src/controller.rs:155-160
- finding: The crate defines `GameVersionStatus` (crd.rs:38-46) but the reconciler patches status with a hand-built `json!({ "status": { "ready": true, "observedGeneration": generation } })`. Field-name drift between the struct's serde renames and the literal JSON keys (e.g. `observedGeneration`) is only caught by eyeball, not the compiler.
- recommendation: Build a `GameVersionStatus { ready: true, message: None, observed_generation: generation }` and serialize it into the patch (`json!({ "status": status })` or `Patch::Merge(serde_json::json!({"status": serde_json::to_value(&status)?}))`).

### weight bound as f64 against a real (float4) column
- severity: minor
- category: quality
- location: operator/src/controller.rs:193
- finding: `.bind(weight as f64)` widens the `f32` spec value to `f64` and sends it as FLOAT8, relying on Postgres's implicit float8 -> float4 assignment cast into `game_types.weight real` (web/migrations/001_initial_schema.sql:146). sqlx encodes `f32` as FLOAT4 natively, so the cast is needless indirection and the test at controller.rs:275 reads the column back as `f32` anyway.
- recommendation: `.bind(weight)` and drop the cast.

### Redundant serde rename on interface_version
- severity: nit
- category: simplicity
- location: operator/src/crd.rs:34
- finding: `#[serde(rename = "interfaceVersion", default = "default_interface_version")]` - the struct already has `#[serde(rename_all = "camelCase")]` (crd.rs:10), which produces `interfaceVersion` for `interface_version`; the explicit rename is redundant.
- recommendation: Keep only `#[serde(default = "default_interface_version")]`.

### Factory wrapped in Arc<Mutex<>> only to smuggle a Fn across threads
- severity: nit
- category: simplicity
- location: tools/fuzz/src/lib.rs:22,31
- finding: `let new_requester = Arc::new(Mutex::new(new_requester));` then `new_requester.lock().unwrap()()` exists only because the bound is `F: Fn() -> R + Send` without `Sync`. Tightening the bound to `F: Fn() -> R + Send + Sync` (both call sites - the `parse_args` closure in tools/fuzz/src/main.rs and `requester::gamer::new::<G>` - satisfy it) removes the mutex and the double-call syntax.
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
- finding: `(player, player_render.clone().command_spec.unwrap(), state)` clones the entire `PlayerRender` (including render strings) to take `command_spec`, immediately after a separate `is_none()` check at line 198.
- recommendation: `let Some(spec) = player_render.command_spec.clone() else { return Err(...) };` and use `spec`.

### interceptor_uri test depends on ambient environment
- severity: nit
- category: quality
- location: operator/src/controller.rs:247-254
- finding: `interceptor_uri_defaults_to_keda_proxy` asserts the default value and relies on `INTERCEPTOR_URI` being unset in the test environment (acknowledged in its comment). Any developer or CI environment that exports the var makes an unrelated test fail.
- recommendation: Either remove the test (it asserts a string literal against itself) or refactor `interceptor_uri` to take an `Option<String>` and test the fallback purely.

### k8s-openapi "latest" feature instead of pinning the cluster's API version
- severity: nit
- category: dependencies
- location: operator/Cargo.toml:14
- finding: `k8s-openapi = { version = "0.28", features = ["latest"] }`. k8s-openapi's guidance is that binaries should enable the version feature matching the oldest cluster they target; `latest` silently moves the compiled-against API surface on every k8s-openapi upgrade. For this operator (only CRD types plus client machinery) the practical risk is low. UNCERTAIN: acceptable if the cluster is kept current with the workspace.
- recommendation: Pin the versioned feature (e.g. `v1_32`) matching the deployed cluster, or document why `latest` is intended.

## Areas reviewed and found clean
- operator/src/main.rs - clean. Crypto-provider install before kube client creation matches the documented workspace constraint; health endpoint semantics are documented and tested; env handling and startup ordering are sound.
- operator/src/controller.rs - beyond the findings above: deletion-path DB update is idempotent and correctly scoped by game_type; observed-generation short-circuit is correct; requeue jitter hack is explicitly justified in a comment (no `rand` dep) and harmless; upserts match the unique constraints (`game_types_name_key`, `game_versions_game_type_id_name_key` verified in web/migrations); `#[sqlx::test]` coverage of the upsert path incl. second-reconcile update is good; error_policy/`for_each` logging matches kube-rs norms.
- operator/src/crd.rs - clean apart from findings above; schema derives, defaults, and camelCase serialization are correct.
- operator/Cargo.toml - clean apart from the k8s-openapi nit; the rustls provider comment is accurate and valuable; reqwest 0.13.4 and sqlx 0.9 confirmed current in Cargo.lock; dev-dependency feature unification for `#[sqlx::test]` is deliberate and commented.
- operator/.mirrord/mirrord.json - trivially fine (incoming off, outgoing ignores localhost).
- tools/fuzz/src/main.rs - clean apart from the panic-inside-worker interaction covered in the hang finding.
- tools/fuzz/src/lib.rs - beyond the findings above: fuzz stepping, state threading (server is stateless, state passed per request so discarded responses cannot drift), seed + command-log repro reporting, and remaining-input handling are all correct.
- tools/fuzz/Cargo.toml - clean apart from num_cpus; rand 0.10.2 confirmed present in Cargo.lock and the `rand::rng()`/`.random()` API usage matches.
- tools/render_plain/src/main.rs - clean. Cross-language markup contract respected (not flagged per brief); `expect` panics are appropriate for a dev tool; player color indexing via `LIGHT.player_color(i)` matches lib/color.
- tools/render_plain/Cargo.toml - clean.
- tools/repl/src/main.rs - clean; minimal wrapper over lib/cmd's repl, `unwrap` on parse_args acceptable for a dev tool.
- tools/repl/Cargo.toml - clean.

## Tally
critical: 0, major: 2, minor: 6, nit: 6
