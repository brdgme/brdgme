# Worker 2 scratch - Tasks A-D

## Task A - sqlx prepared-query cache: REFUTED (correct consolidation)

Verdict: **the `rust/.sqlx` deletion in WP-52 (`f374434`) is correct cleanup, not a hazard.**

Evidence:

```
$ find . -maxdepth 4 -name .sqlx -type d -not -path './.git/*'
./rust/web/.sqlx          # 137 query-*.json at HEAD

$ git ls-tree -r --name-only f374434^ | rg '^rust/\.sqlx/' | wc -l      -> 81
$ git ls-tree -r --name-only f374434^ | rg '^rust/web/\.sqlx/' | wc -l  -> 135
$ git ls-tree -r --name-only HEAD     | rg '^rust/\.sqlx/' | wc -l      -> 0
$ git ls-tree -r --name-only HEAD     | rg '^rust/web/\.sqlx/' | wc -l  -> 137
```

1. Ordering is the reverse of the premise: `f374434` (WP-52, 2026-07-28) is an
   **ancestor** of `667c8f42` (WP-66, 2026-07-29), so the deletion is not
   fallout from the sqlx 0.9 unification. WP-66 then regenerated **only**
   `rust/web/.sqlx` (88 files; `git show --numstat 667c8f42 | rg '\.sqlx/'`
   yields 88 paths, all under `rust/web/.sqlx`). Had `rust/.sqlx` survived
   WP-52 it would today be an 81-entry sqlx-0.8-format orphan.
2. Only `web` uses compile-time macros. `rg 'query(_as|_scalar)?!' bot/src
   operator/src lib/session_store/src tools/fuzz/src` -> **zero hits**; `web`
   has 16 files. `bot`/`operator`/`session_store` use runtime
   `sqlx::query(...)`/`.bind()`, which needs no offline cache. So the
   workspace-root fallback (`$WORKSPACE_ROOT/.sqlx`) is never consulted:
   `web` resolves `$CARGO_MANIFEST_DIR/.sqlx` = `rust/web/.sqlx` first.
3. `cargo sqlx prepare` is always invoked from `rust/web`, never with
   `--workspace`, so it writes `rust/web/.sqlx`:
   - `scripts/rust-ci-commands.sh:24` -
     `(cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`
   - `docs/DEV.md:73,94`, `Tiltfile:147` - same, from `rust/web`.
   - There is no `.cargo/config.toml` anywhere in the tree.
4. CI would catch staleness. `.github/workflows/ci.yml:52` sets
   `SQLX_OFFLINE: "true"` for the job and line 94 runs
   `bash ../scripts/rust-ci-commands.sh`, which does the
   `cargo sqlx prepare --check` above plus clippy/tests under `SQLX_OFFLINE`.
   `rust/Dockerfile:76` also sets `ENV SQLX_OFFLINE=true`.

No finding. Note only: WP-52's commit message (`perf(web): WP-52 stats and query
performance pass`) does not mention removing an 81-file directory - a Low
process nit, not a code defect.

## Task B - `test-support` feature consumers: all correct

`rg -n 'test-support' --glob '**/Cargo.toml' rust/` -> 29 hits:
- `rust/lib/cmd/Cargo.toml:26` - the feature declaration `test-support = []`.
- 28 game crates, each exactly
  `brdgme_cmd = { path = "../../lib/cmd", features = ["test-support"] }`.

Section check (awk over each `game/*/Cargo.toml`, printing the last `[section]`
header before the match): **28/28 are under `[dev-dependencies]`. Zero under
`[dependencies]`.** Nothing ships in a release build.

`assert_gamer_contract`: 29 files - `rust/lib/cmd/src/test_support.rs`
(definition) + 28 `rust/game/*/tests/contract.rs`. That is every game crate in
the workspace (`rust/Cargo.toml` lists 28 `game/` members). No coverage gap.

## Task C - WP-62 operator finalizer race (`e682f6bc`)

Spec exists: `docs/reviews/2026-07-23-rust-review/planning/specs/WP-62-operator.md`
(recovered at `868094a6`).

### Verified good

- **bo F18 (the race) is genuinely closed.** `rust/operator/src/controller.rs:77`
  dispatches through `kube::runtime::finalizer::finalizer(&api, FINALIZER, obj, ...)`
  and both hand-rolled `Patch::Merge(json!({"metadata":{"finalizers": ...}}))`
  calls are gone. Confirmed the helper is conflict-safe by reading the vendored
  crate: `~/.cargo/registry/.../kube-runtime-4.0.0/src/finalizer.rs:160-207` uses
  `Patch::Json` with `PatchOperation::Test` guards -
  `// Test ensures that we fail instead of deleting someone else's finalizer`.
  Merge-patch array clobbering is eliminated.
- **Ordering matches the consumer exactly.** The guarded update's
  `(newer.created_at, newer.name) > (cur.created_at, cur.name)` on
  `is_deprecated = false` (controller.rs:249-260) is the precise inverse of
  `rust/web/src/db/game_types.rs:37-38`
  (`WHERE game_type_id = $1 AND is_deprecated = false ORDER BY created_at DESC, name DESC`).
  No divergence in the tie-break.
- **No pattern-2 sibling.** `interceptor_uri` has one caller (controller.rs:116);
  both status patches (reconcile failure at :92, apply success at :164) use the
  typed `GameVersionStatus`; `weight` is bound once, no `as f64` remains.
- **No `#[allow(dead_code)]` / zero-caller additions** in this commit.
- **Not decoy tests.** `authoritative_version_wins_regardless_of_order_deprecated_first`
  (controller.rs:400) writes `[2]/1.0/"old blurb"` via the initial INSERT, then
  `[2,3]/2.0/"new blurb"` via the guarded UPDATE, then re-upserts the deprecated
  version and asserts the row is unchanged. Under the pre-fix
  `ON CONFLICT ... SET player_counts = EXCLUDED...` this third step would revert
  the row and the assertion at :465 would fail. Genuine regression test.
- `k8s-openapi` `v1_36` feature confirmed to exist
  (`k8s-openapi-0.28*/Cargo.toml:61`, and `latest = ["v1_36"]` at :53, so the pin
  is a real freeze against future drift).
- `crd.rs` riders landed: no `.spec.playerCounts` printcolumn (crd.rs:17), no
  redundant `rename = "interfaceVersion"` (crd.rs:33-34).

### Finding C1 - Medium - `/home/beefsack/Development/brdgme/rust/operator/src/controller.rs:240`

**What.** The authoritative-version guard only ever *writes forward*. It runs
`if !is_deprecated`, and `apply` short-circuits on an unchanged generation
(`controller.rs:111-114`):

```rust
if generation.is_some() && generation == observed_generation {
    info!(name, "Spec unchanged since last reconcile, skipping");
    return Ok(requeue_with_jitter());
}
```

So when the *newest* version is flipped to `isDeprecated: true`, nothing ever
rewrites `game_types` back to the now-authoritative older version.

**Concrete failing path.**
1. `lost-cities-1` and `lost-cities-2` both non-deprecated; `game_types` holds
   `-2`'s `player_counts = [2,3]`.
2. Edit the `lost-cities-2` CR to `isDeprecated: true`. Its generation bumps,
   `apply` runs, `game_versions.is_deprecated` becomes true, and the guarded
   UPDATE at :240 is skipped because `is_deprecated`. `game_types` keeps `[2,3]`.
3. `find_latest_non_deprecated_game_version` (`rust/web/src/db/game_types.rs:28`)
   now returns `lost-cities-1`, but `find_game_type_player_counts`
   (`game_types.rs:49-57`) reads the shared **type** row and still returns `[2,3]`.
4. `lost-cities-1`'s own generation never changes, so its `apply` keeps hitting
   the early return at :111 forever - the hourly requeue does not repair it.
   A 3-player Lost Cities is accepted by web validation and then rejected by the
   game service. This is exactly the failure the spec's "Design call" section
   set out to prevent ("makes `game_types` describe precisely what
   `find_latest_non_deprecated_game_version` will hand out").

**Fix.** Run the descriptive-column update unconditionally for the winning
version rather than gating on `!is_deprecated` on the *reconciling* CR - i.e.
resolve the authoritative row inside SQL (`ORDER BY created_at DESC, name DESC
LIMIT 1` over non-deprecated versions of the type) and copy its values, so a
deprecation flip on any version re-points the type row. Same treatment covers
`cleanup` (which sets `is_public = false` but leaves `is_deprecated = false`, so
a deleted newest version still blocks older ones from reclaiming the type row).

### Coverage gap C2 - Low - controller.rs tests

The four DB tests cover "newer non-deprecated version wins" in both insertion
orders and the deprecated-only first write, but **no test flips an already-newest
version to deprecated**, nor exercises `cleanup` at all (`cleanup` at
controller.rs:174 has zero test callers). Add a test for the C1 sequence.

## Task D - WP-63 fuzz hang (`d2decf85`): fixed, not merely mitigated

Spec: `.../specs/WP-63-fuzz-tool.md`. Every item (3a-3f) landed verbatim.

**The hang is genuinely closed for the stated trigger.** The bug was that the
driver held its own `step_tx`, so the channel never disconnected when all workers
panicked. Final code, `rust/tools/fuzz/src/lib.rs:47`:

```rust
drop(step_tx);
```

plus `lib.rs:85-89`:

```rust
Err(_) => {
    eprintln!("{}", tally.render());
    eprintln!("all fuzz workers exited");
    break;
}
```

That is the complete fix, not a bound-on-an-unbounded-loop. No timeout is
involved anywhere, so there is no partial-coverage timeout to critique.

`fuzz_returns_when_all_workers_exit` (lib.rs:~370) is **not** a decoy: its
`StubRequester::request` returns `Err(RequestError::Stdin)`, which makes
`Fuzzer::try_new` fail, which makes every worker panic at
`.expect("expected to create fuzzer")` (lib.rs:35). Without `drop(step_tx)` the
driver blocks in `recv()` and the `recv_timeout(10s)` assertion fails. It does
call the function under test and its input does not pass independently.

Residual, all **out of scope by explicit spec non-goal** ("Do not restructure
worker shutdown beyond 3a/3e ... the workers' own `send(...).expect(...)` on a
dropped receiver stays as-is"), recorded for completeness only:

- Low - `lib.rs:37-39`: after the driver breaks on `FuzzStep::Error` and returns,
  surviving workers panic on `step_tx.send(...).expect("failed to send fuzz step")`
  against the dropped receiver. Noisy stderr on every real find; not a hang.
- Low - `lib.rs:53-57`: the 1s tally render is only reached after `recv()`
  returns, so a fully-blocked worker set produces no periodic output. A worker
  wedged inside `requester.request()` (child binary alive but silent) still hangs
  the driver - `recv()` has no timeout. Different trigger from bo F26; not
  claimed fixed by this WP. Would need `recv_timeout` to close.
- `lib.rs:38` `fuzzer.next().expect(...)` is unreachable-`None`:
  `Iterator::next` (lib.rs:253-291) returns `Some` on every path.
