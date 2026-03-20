# Current Work Status

## Phase 5.6: In Progress

All 13 blockers resolved. All 4 missing API endpoints implemented.
New-game creation UI complete. Operator built and running.
Frontend gaps and code quality items remain (see PLAN.md).

---

## Completed this session

### Kubernetes operator (`rust/operator`)

- Built `brdgme-operator`: kube-rs operator watching `GameVersion` CRDs.
- `GameVersion` CRD defined in `k8s/base/operator/crd.yaml`.
- Operator upserts `game_types` and `game_versions` rows in PostgreSQL on
  reconcile. Uses finalizers to set `is_public = false` on deletion.
- `is_deprecated` field on `GameVersion` spec - used for `lost-cities-1`
  which remains running for in-progress games but cannot start new ones.
- Migration `003_game_type_constraints.sql`: unique constraints on
  `game_types(name)` and `game_versions(game_type_id, name)` for upserts.
- 20 `GameVersion` CR YAML files colocated with each game in
  `k8s/base/game/{name}/game-version.yaml`.
- Operator runs as a `local_resource` in Tilt hybrid mode with
  `RUST_LOG=info` for visible reconcile output.
- `crd-ready` Tilt local_resource uses `kubectl wait --for=condition=established`
  to gate operator startup on CRD registration.

### New-game creation UI

- `GamesPage` implemented: game type selector, optional version selector,
  player count selector, opponent email inputs, submit → redirect to new game.
- `get_available_game_types` server function queries `game_types` joined with
  non-deprecated `game_versions`.
- `create_new_game` server function: calls game service, creates DB records,
  broadcasts WebSocket update, returns new game ID.
- SQLx offline metadata regenerated after adding new queries.

### Dev environment improvements

- `secret_settings(disable_scrub=True)` in Tiltfile: prevents Tilt from
  redacting "brdgme" (the DB name) from all log output.
- `scripts/setup-kind-cluster.sh`: made idempotent (guards `kind create
  cluster`); fixed Kourier webhook race condition by polling endpoint
  registration instead of using `rollout status`.
- Dev login: login codes now printed to stdout via `println!` when SMTP is
  unavailable, making local testing straightforward.
- `mirrord` added to `devenv.nix`; Tiltfile wraps `cargo leptos watch` with
  `mirrord exec --target pod/postgres-0 --target-namespace brdgme` so the
  local web server can resolve `*.svc.cluster.local` DNS without any
  application-level hacks or `/etc/hosts` modification.

---

## Immediate next tasks (Phase 5.6 frontend gaps)

1. **Game log rendering** - replace `GameLogs` stub with actual log display.
2. **Undo/concede/restart actions** - wire buttons in `GameMeta`.
3. **"Whose turn" display** - show specific player name(s) and color.
4. **Mark-read on game page load** - call `mark_read` when `GamePage` mounts.
5. **`GameRestarted` WebSocket navigation** - navigate to new game URL on receipt.
6. **Command input UX** - clear after confirmation, surface errors, clickable suggestions.
7. **Autocomplete prefix filtering** - add `CommandSpec::suggest()`.
