# Current Work Status

## Phase 5.6: In Progress - 10 of 13 blockers complete, build working

---

## This session: completed work

### Dev environment fixes

**Kind cluster recreation failure**
- Root cause: previous kind version defaulted to K8s v1.35.0 which had a
  kubelet startup bug. New kind v0.30.0 defaults to v1.34.0 - works correctly.
- Fix: deleted and recreated cluster with `bash scripts/setup-kind-cluster.sh`.

**KEP-1755 ConfigMap corrected** (`scripts/setup-kind-cluster.sh`)
- Was: `host: "kind-registry:5000"` (in-cluster address, caused Tilt to try
  pushing from the host to an unresolvable hostname).
- Fixed to: `host: "localhost:5000"` + `hostFromContainerRuntime: "kind-registry:5000"`.
- Tilt reads the `host` field to know where to push images from the host.

**`wasm-bindgen-cli` missing from devenv** (`devenv.nix`, `rust/web/Cargo.toml`)
- `cargo-leptos watch` requires `wasm-bindgen` binary in PATH.
- Previously installed ad-hoc via `cargo install`; devenv rebuild wiped it.
- Fix: added `wasm-bindgen-cli` to `devenv.nix` packages.
- Version bump: nixpkgs has 0.2.108; Cargo.toml pin updated from `=0.2.100`
  to `=0.2.108` to match. Cargo.lock updated via `cargo update -p wasm-bindgen`.

**`rust/.gitignore` created**
- `cargo-leptos`'s file watcher complained about missing `.gitignore`, causing
  it to watch `target/` and making recompile detection slow.

### chrono -> time migration

`tower-sessions-sqlx-store 0.15.0` hard-codes `sqlx/time` in its dependencies.
When both `time` and `chrono` SQLx features are active, `query_as!` picks
`time::PrimitiveDateTime` for `TIMESTAMP` columns and tries to convert to the
struct field type via `From`. `NaiveDateTime: From<PrimitiveDateTime>` is not
implemented, so every `query_as!` call failed to compile.

Changes made:
- `rust/lib/game/Cargo.toml`: `chrono` -> `time = { version = "0.3", features = ["serde"] }`
- `rust/lib/cmd/Cargo.toml`: same
- `rust/lib/game/src/game_log.rs`: `Log.at` changed from `chrono::NaiveDateTime`
  to `time::PrimitiveDateTime`; constructors use `OffsetDateTime::now_utc()`
- `rust/lib/game/src/bot.rs`: `chrono::Utc::now().timestamp()` replaced with
  `std::time::Instant` (no time crate needed for a 1-second interval check)
- `rust/lib/cmd/src/api.rs`: `CliLog.at` changed to `time::PrimitiveDateTime`
- `rust/api/src/db/query/mod.rs`: conversion added at `logged_at: l.at` site
  (old Rocket API uses Diesel which is chrono-based; conversion keeps it working)
- `rust/web/src/models/*.rs`: all `chrono::NaiveDateTime` -> `time::PrimitiveDateTime`
- `rust/web/src/auth/server.rs`: `Utc::now().naive_utc()` ->
  `OffsetDateTime::now_utc()` + `PrimitiveDateTime::new(now.date(), now.time())`
- `rust/web/Cargo.toml`: `chrono` removed, `time = { version = "0.3", features = ["serde"] }` added

### tokio/signal feature added (`rust/web/Cargo.toml`)

`tokio::signal` requires the `signal` feature. Was missing, causing `main.rs`
`shutdown_signal()` to fail to compile. Added to the `tokio` dependency.

### SQLx offline metadata regenerated

`cargo sqlx prepare` now run from `rust/web/` (not workspace root).
Output: `rust/web/.sqlx/` (29 files). Must be committed.
Previous location `rust/.sqlx/` is now empty (the workspace-level prepare
finds no queries because all queries are in the `web` crate).

---

## Remaining Phase 5.6 blockers (not yet coded)

In order of recommended priority:

1. **`GamePlayer` model missing fields** (`models/game.rs`):
   Add `last_turn_at`, `is_eliminated`, `is_read`, `points`, `undo_game_state`,
   `rating_change`. Required before undo, mark_read, and points work.

2. **`update_game_command_success` writes all fields** (`db.rs`):
   Persist `is_turn_at`, `last_turn_at`, `is_eliminated`, `undo_game_state`,
   and points on every command. Also set `finished_at` when `is_finished = true`.

3. **`find_game_extended` handles missing `game_type_users` row** (`db.rs`):
   Use LEFT JOIN with a default rating (1500) rather than erroring.

4. **Email sending** (`auth/server.rs`):
   Send confirmation token via in-cluster SMTP service. SMTP pod is deployed.
   Use `lettre` (more actively maintained than the `email` crate already in
   Cargo.toml; consider switching). Read SMTP host/port from env.

---

## What to do next

Start with blocker 1 (GamePlayer missing fields) - it unblocks blockers 2 and
the undo/mark_read endpoints. After adding fields to the model, re-run:

```bash
cd rust/web && cargo sqlx prepare -- --features ssr
```

The migration also needs updating since the schema lacks these columns. Add a
new migration file `rust/web/migrations/002_game_player_fields.sql` with ALTER
TABLE statements for each new column.

---

## Summary of all Phase 5.6 blocker status

| # | Blocker | Status |
|---|---------|--------|
| 1 | Persistent session store | Done |
| 2 | Login UI wired | Done |
| 3 | Token not in response | Done |
| 4 | `with_secure` env-driven | Done |
| 5 | Token expiry 30-day | Done |
| 6 | Email sending | Not started |
| 7 | Auth in Axum handlers | Done |
| 8 | Authenticate GET /game/:id | Done |
| 9 | Turn enforcement | Done |
| 10 | GamePlayer missing fields | Not started |
| 11 | update_game_command_success all fields | Not started |
| 12 | find_game_extended LEFT JOIN | Not started |
| 13 | Graceful SIGTERM | Done |
