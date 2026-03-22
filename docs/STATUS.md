# Current Work Status

## Phase 5.6: In Progress

All 13 blockers resolved. All 4 missing API endpoints implemented.
New-game creation UI complete. Operator built and running.
Frontend gaps and code quality items remain (see PLAN.md).

---

## Completed this session (2026-03-21, continued)

### Dev environment: Cilium -> Kourier revert

- Reverted from `leptos-cilium-dev` branch changes. Cilium has three
  independently broken paths for Kind + Knative (NodePort, host network,
  L2 announcements). Kourier is the only officially supported ingress for
  Kind + Knative and has none of these blockers.
- `k8s/kind-config.yaml`: removed Cilium flags, added `extraPortMappings`
  containerPort 31080 -> hostPort 8080 for Kourier NodePort.
- `scripts/setup-kind-cluster.sh`: replaced Cilium/Gateway API with Kourier
  install + NodePort patch + `lvh.me` domain config. Fixed `rollout status`
  namespace bug (`net-kourier-controller` is in `knative-serving`).
- `k8s/base/postgres/service.yaml`, `k8s/base/redis/service.yaml`:
  `NodePort` -> `ClusterIP`.
- `k8s/base/postgres/stateful-set.yaml`: `postgres:12.3` -> `postgres:18`
  (PostgreSQL 12 reached EOL November 2024).
- Research notes in `docs/CILIUM-KNATIVE-KIND-TILT-DEV.md`.

### Migration system overhaul

- Removed `sqlx::migrate!().run()` auto-migration from `create_pool()` in
  `rust/web/src/db.rs`. Migrations now run explicitly only.
- `rust/Dockerfile`: added `migrate-builder` and `migrate` stages. The
  `migrate` image runs `sqlx migrate run` and is intended as a pre-deploy
  ArgoCD Job.
- `k8s/base/web/migrate-job.yaml`: ArgoCD pre-sync Job manifest (ready for
  Phase 6.5 ArgoCD setup).
- `Tiltfile`: added `DATABASE_URL` to postgres-config secret; added manual
  `migrate` local_resource for dev workflow.

### Migration 001 comprehensive rewrite

Exhaustively compared `001_initial_schema.sql` against the production schema
(taken from `brdgme-2026-03-21.dump`). Migration now exactly replicates prod:
- Added 4 missing functions: `update_updated_at`, `update_finished_at`,
  `update_is_turn_at`, `update_last_turn_at`.
- Added all `game_players` columns missing from original 001 (previously in
  002): `last_turn_at`, `is_eliminated`, `is_read`, `points`,
  `undo_game_state`, `rating_change`.
- Added 7 missing unique constraints: `users_name_key`, `user_emails_email_key`,
  `game_type_users_game_type_id_user_id_key`,
  `game_versions_game_type_id_name_key`, `game_players_game_id_color_key`,
  `game_players_game_id_position_key`, `game_players_game_id_user_id_key`.
- Added all 17 `update_*_updated_at` triggers and 3 conditional triggers
  (`update_finished_at`, `update_is_turn_at`, `update_last_turn_at`).
- Removed Diesel ORM artifacts: `diesel_set_updated_at`,
  `diesel_manage_updated_at`, all `set_updated_at` triggers. Standardised on
  `update_updated_at()` trigger pattern throughout.
- Removed `__diesel_schema_migrations` (Diesel internal, not needed).
- `002_game_player_fields.sql` deleted (fully absorbed into 001).
- `003_game_type_constraints.sql` renamed to `002_game_type_constraints.sql`.
  Now only adds `game_types_name_key` (`game_versions_game_type_id_name_key`
  already existed in prod, moved to 001).
- Verified against prod data: both migrations apply cleanly, schema diff shows
  only intentional additions (indexes, `game_types_name_key`, Diesel cleanup).

### CI fixes and optimisation

- `rust/web/src/lib.rs`: added `#![recursion_limit = "512"]` - 42 SQLx query
  macros were overflowing the default limit of 128.
- `rust/Dockerfile`: upgraded `cargo-leptos` from `0.2.42` to `0.3.5` to match
  local devenv (0.2.42 bundled wasm-bindgen-cli 0.2.100, incompatible with the
  project's wasm-bindgen 0.2.108 pin).
- `rust/Dockerfile`: added explicit `cargo binstall wasm-bindgen-cli --version
  0.2.108` to pin to project version.
- `rust/Dockerfile`: bumped dart-sass from `1.77.8` to `1.97.3` to match local.
- `rust/Dockerfile`: pinned `sqlx-cli` to `0.8.6` (was unpinned).
- `.github/workflows/ci.yml`: collapsed `build-rust-games` matrix (3 games)
  into a single sequential job - Docker's local buildx cache shares the
  `builder` stage across game targets, compiling the workspace once instead of
  three times.
- `.github/workflows/ci.yml`: collapsed `build-go-games` matrix (17 games)
  into a single sequential job with a shell loop - `go-builder` compiles once,
  all 17 games reuse it locally.
- `.github/workflows/ci.yml`: added `cache-from/cache-to: type=gha` to
  `test-go` (was the only job not priming the GHA Docker cache).

---

## Immediate next tasks

Next phase: **Phase 6** - Redis pub/sub to replace `tokio::sync::broadcast`.
Required for multi-replica correctness and side-by-side validation.
See PLAN.md Phase 6.

---

## Completed this session (2026-03-22)

### Concede confirmation

- `components/game.rs`: Added `window.confirm("Are you sure you want to concede?")`
  before dispatching `ConcedeGame` action. Added `"Window"` to web-sys features
  in `Cargo.toml`.

### Restart error diagnostics

- `game/client.rs`: `client::request` now reads the response body as text first,
  then parses JSON. On failure the raw body is included in the error message:
  "error parsing response: {raw body}". This will expose the actual game service
  response on the next restart attempt so the root cause can be identified.
  (Previously the error was "error parsing JSON response" with no further detail.)
