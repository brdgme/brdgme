# WP-62: operator

**Findings:** bo F18 (major), bo F19-F22 (minor), bo F23-F25 (nit), plus one
**routed-in major** (`game_types` last-writer-wins) ruled into this package by
WP-28's lead ruling. **Findings source:** the raw
`findings/bot-operator-tools.md` - there is no
`findings/verification/bot-operator-tools.md` (unit was lead-verified).

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

- **bo F18** - `reconcile` (`rust/operator/src/controller.rs`) hand-rolls
  finalizer add/remove and writes the whole array back with `Patch::Merge`.
- **ROUTED-IN MAJOR** - `upsert_game_type_and_version`
  (`rust/operator/src/controller.rs`) writes `player_counts` / `weight` /
  `blurb` onto the `game_types` row keyed by type name. Every `GameVersion` CR
  of the same type writes that one row, last-reconcile-wins.
- **bo F19** - `GameVersion`'s derive (`rust/operator/src/crd.rs`) declares a
  `Players` printcolumn at `.spec.playerCounts`; `GameVersionSpec` has no such
  field.
- **bo F20** - `GameVersionStatus.message` is never written; a failing
  reconcile leaves no status at all, or a stale `ready: true`.
- **bo F21** - the success path patches status with a hand-built `json!`
  instead of the typed `GameVersionStatus`.
- **bo F22** - `upsert_game_type_and_version` binds `weight as f64` against a
  `real` column.
- **bo F23** - `GameVersionSpec::interface_version` carries a redundant
  `#[serde(rename = "interfaceVersion")]`.
- **bo F24** - `interceptor_uri_defaults_to_keda_proxy` depends on
  `INTERCEPTOR_URI` being unset in the ambient environment.
- **bo F25** - `k8s-openapi` uses the `latest` feature. **ANSWERED**, see §4.

## 2. Why it's wrong

- **bo F18 is correct as written.** Verified live: both patches are
  `Patch::Merge(json!({ "metadata": { "finalizers": ... } }))` built from the
  watch-cache copy. Merge patch replaces the array wholesale, so a concurrent
  finalizer add is silently dropped and a concurrent removal is resurrected.
- **Routed-in major is correct.** Verified live: the upsert is
  `ON CONFLICT (name) DO UPDATE SET player_counts = EXCLUDED.player_counts, ...`.
  New games pick a version via
  `rust/web/src/db.rs::find_latest_non_deprecated_game_version`, but validate the
  roster against the *type-level* counts via
  `rust/web/src/db.rs::find_game_type_player_counts`. So the type row must
  describe exactly the version new games will actually start.
- **bo F19 is correct that the field does not exist**, but its stated symptom is
  wrong: the applied CRD `k8s/base/operator/crd.yaml` lists only the
  `Display Name` column, so no empty column is rendered today. The real defect
  is derive-vs-manifest drift.
- **bo F20, F21, F22, F23, F24 are correct as written.** All verified live.

**Design call for the routed-in major: take the newest non-deprecated version's
values; do NOT union.** A union would advertise player counts that the version
new games actually start cannot run - `find_game_type_player_counts` is keyed by
version id but returns the shared type row, so a 3-player roster on a union'd
Lost Cities would be accepted by validation and then rejected by the game
service. Taking the newest non-deprecated version makes `game_types` describe
precisely what `find_latest_non_deprecated_game_version` will hand out. It also
fixes the Lost Cities case (`lost-cities-1` is `isDeprecated: true`, so `-2`'s
`[2, 3]` wins deterministically instead of by reconcile order).

## 3. Required end state

**Crate span:** everything below is inside `rust/operator/`.
`upsert_game_type_and_version` lives in `rust/operator/src/controller.rs`, not in
`rust/web`. Do not edit `rust/web/` or `k8s/`.

### 3a. `controller.rs::reconcile` - use the kube-rs finalizer helper (bo F18)

Split the body into `apply(obj, ctx)` and `cleanup(obj, ctx)` free fns (cleanup
holds the existing `UPDATE game_versions SET is_public = false ...`), and make
`reconcile` dispatch through `kube::runtime::finalizer::finalizer(&api,
FINALIZER, obj, |event| ...)` matching on `Event::Apply` / `Event::Cleanup`.
Delete both hand-built finalizer merge patches. Add a boxed variant to `Error`
for `kube::runtime::finalizer::Error<Error>` (box it - the type is recursive).
`kube-runtime` 4.0.0 is in the lockfile with the `runtime` feature enabled;
**confirm the `finalizer` module path before editing** and STOP if it differs.

### 3b. `controller.rs::upsert_game_type_and_version` - authoritative version only

Three statements, in this order:

1. `INSERT INTO game_types (name, player_counts, weight, blurb) VALUES (...)
   ON CONFLICT (name) DO UPDATE SET updated_at = NOW() RETURNING id` - the
   conflict branch must no longer touch the three descriptive columns.
2. The existing `game_versions` upsert, unchanged.
3. A guarded `UPDATE game_types SET player_counts = $, weight = $, blurb = $,
   updated_at = NOW() WHERE id = $` that runs only when this CR is not
   deprecated **and** no other non-deprecated `game_versions` row of the same
   `game_type_id` is newer (`created_at`, tiebreak `name`, both descending).
   Verify those columns exist before writing the SQL.

Bind `weight` as `f32`, no `as f64` (bo F22).

### 3c. `controller.rs` - typed status, and status on failure (bo F20, F21)

Build a `crate::crd::GameVersionStatus` value and patch
`Patch::Merge(json!({ "status": status }))`; drop the hand-written key literals.
Success writes `ready: true, message: None, observed_generation: generation`.
Wrap the `finalizer(...)` call in `reconcile`: on `Err`, best-effort patch
`ready: false, message: Some(err.to_string()), observed_generation: generation`
before returning the error - log and swallow a failed status patch (the object
may already be gone). `error_policy` keeps only its log + 30s requeue.

### 3d. `crd.rs` (bo F19, F23)

Delete the `.spec.playerCounts` printcolumn so the derive matches
`k8s/base/operator/crd.yaml`. Drop the redundant `rename = "interfaceVersion"`,
keeping `#[serde(default = "default_interface_version")]`.

### 3e. `controller.rs::interceptor_uri` (bo F24)

Change to `interceptor_uri(env: Option<String>) -> String` (callers pass
`std::env::var("INTERCEPTOR_URI").ok()`) and test the `None` fallback purely.

## 4. Non-goals

- **bo F25 is ANSWERED** (`decisions-ANSWERED.md`, `bo F25` row): the deployed
  cluster is Kubernetes server **v1.36.0**. Replace `features = ["latest"]` on
  `k8s-openapi` in `rust/operator/Cargo.toml` with **`v1_36`**. The implementer
  must confirm `k8s-openapi` actually ships a `v1_36` flag at fix time; if it
  does not, use the **highest available flag at or below v1.36** and record the
  choice here. Do not run `kubectl`.
- No other `rust/operator/Cargo.toml` changes: workspace-deps migration (WP-64),
  sqlx unification (WP-66) and sentry feature trim (WP-67) are
  BLOCKED-ON-DECISION and own that file.
- No edits to `rust/web/` (including `players.rs`, owned by WP-47), `k8s/`, or
  any migration. No `bot/` or `tools/fuzz` findings. No change to
  `find_latest_non_deprecated_game_version` or its tie-break semantics.
- Do not add `playerCounts` to `GameVersionStatus` - that would require editing
  the applied CRD manifest.

## 5. Regression test cases

All in the existing `#[cfg(test)] mod tests` in
`rust/operator/src/controller.rs` (it already has `#[sqlx::test(migrations =
"../web/migrations")]` DB tests - extend it, do not add a new file).

- **Authoritative version (routed-in major):** one type, two versions. Upsert a
  deprecated version reporting `[2]` and a non-deprecated one reporting
  `[2, 3]`; assert `game_types.player_counts` is `[2, 3]` **in both reconcile
  orders**, and that the same holds for `blurb` and `weight`. Then upsert the
  deprecated version again and assert the row is unchanged.
- **First-write path:** a brand-new type whose only version is deprecated still
  gets its counts/weight/blurb written by the initial `INSERT`.
- **weight:** extend `upsert_writes_weight_and_blurb` with a value that is not
  exactly representable in decimal (e.g. `2.7f32`) and assert the `f32` read-back
  is bit-equal.
- **Status shape (bo F21, F23):** assert `serde_json::to_value` of a
  `GameVersionStatus` produces exactly the keys `ready` / `observedGeneration`
  (and `message` only when `Some`), and that deserializing
  `{"typeName":"X","interfaceVersion":2}` into `GameVersionSpec` yields
  `interface_version == 2` while an absent key yields `1`.
- **`interceptor_uri` (bo F24):** `None` returns the KEDA default; `Some("x")`
  returns `"x"`. No environment access in the test.

## 6. Riders

| File | One-line fix | Test needed |
| --- | --- | --- |
| `rust/operator/src/crd.rs` (bo F19) | Delete the `.spec.playerCounts` printcolumn | n |
| `rust/operator/src/crd.rs` (bo F23) | Drop the redundant `rename = "interfaceVersion"` | y (serde shape test) |
| `rust/operator/src/controller.rs` (bo F22) | `.bind(weight)`, drop `as f64` | y |
| `rust/operator/src/controller.rs` (bo F24) | `interceptor_uri` takes `Option<String>` | y |
| `rust/operator/Cargo.toml` (bo F25) | **ANSWERED** (`decisions-ANSWERED.md`) - cluster is k8s v1.36.0; pin `k8s-openapi` `v1_36`, or the highest flag at or below v1.36 if `v1_36` is absent, recording the choice | n |
