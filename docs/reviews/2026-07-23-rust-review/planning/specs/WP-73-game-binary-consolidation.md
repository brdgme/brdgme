# WP-73: game binary consolidation

**Findings:** dp F11, dp F26, e F45, e F46. **Decisions:** D-20 - a generic bin
crate parameterised over `Gamer` plus thin per-game wrappers, explicitly **NOT**
a macro (Michael approved it partly *because* it avoids macros;
`brdgme_game_bin` must stay macro-free, and this spec introduces zero macros).
D-41 + D-43 - delete the 27 per-game `_repl` bins only; the 27 `_fuzz` bins and
`fuzz_gamer` SURVIVE (3d). D-42 - see 4.
**Landing order:** after WP-64 - see 6.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

Built on `planning/raw/wp73-game-bin-inventory.md` (Dockerfile, workspace and
manifest claims independently confirmed) and
`planning/raw/wp73-fuzz-repl-dependency-sweep.md` (3d), as amended for fuzz by
`planning/fuzz-throughput-evaluation.md` (Lead-ACCEPTED; its recommendation is
binding here). *Over the ~120-line Tier 2 cap (Lead-accepted): 3a must not be
compressed, and the package touches 27 crates plus a 27-file deletion.*

## 1. Problem

27 game crates x 4 bins = 108 files, ~39 lines each, semantically identical
modulo `use <crate>::Game;`. Each crate declares `brdgme_cmd`, `brdgme_fuzz` and
`tokio = { features = ["full"] }` as library deps purely to serve those bins, so
the workspace compiles tokio-"full" 27 times; the http bin also defaults to a
privileged port. The 27 `_repl` bins duplicate the generic out-of-process tool
and are deleted here (3d); the 27 `_fuzz` bins stay for throughput reasons
(D-43), leaving 81 three-line wrappers.

## 2. Findings verdicts - state these so nobody reverts them

- **dp F11 CORRECT.** All 27 manifests carry byte-identical
  `tokio = { version = "1.52.3", features = ["full"] }` for a 13-line http bin.
  Resolved: tokio leaves the game crates entirely.
- **dp F26 CORRECT.** Counts and "only variation is the crate name" verified.
- **e F45: facts CORRECT, recommendation INVALID - do not apply it.** Cargo does
  **not** link `[dev-dependencies]` into `src/bin/` targets, so that move breaks
  every bin (`findings/verification/games-batch-e.md` records it as ADJUSTED).
  Resolved differently: the deps move to `game_bin`.
- **e F46 CORRECT, sharper than written.** The distroless stages run
  `USER 65532`, so the `0.0.0.0:80` default is unusable in the shipped image; it
  works only because `k8s/base/game/*/deployment.yaml` sets `ADDR=0.0.0.0:8080`.
  Default to `0.0.0.0:8080` - containerPort and both probes are already 8080.

## 3. Required end state

### 3a. DO NOT RENAME OR RELOCATE THE SURVIVING BIN FILES

`rust/Dockerfile` has 26 distroless stages doing
`COPY --from=builder /app/target/release/<snake>_http .` and
`CMD ["./<snake>_http"]`. `rust/tools/fuzz` and `rust/tools/repl` are pointed at
a `_cli` binary **by path** at runtime. Cargo derives the bin target name from
the `src/bin/` file name. **Renaming, moving, or converting any `_http` or
`_cli` bin to a differently named target breaks game images or the generic
tools.** After the 3d deletion, keep all 81 surviving paths exactly as they are:
`rust/game/<game>/src/bin/<snake>_{cli,http,fuzz}.rs`. Target names must also stay
globally unique across the workspace - the `COPY` is by flat filename out of
`target/release/`. No `[[bin]]` stanzas exist today; none are needed.

### 3b. New crate `rust/lib/game_bin`, package `brdgme_game_bin`

(snake_case dir + `brdgme_<snake_dir>` is the `lib/` convention; hyphens are the
game-crate convention.)

`src/lib.rs` exposes exactly **three** generic no-argument functions, each body
the **current body of the corresponding per-game bin, moved verbatim**. No
`repl_main` - D-41 deleted those bins.

- `cli_main` - `brdgme_cmd::cli::cli(&mut requester::gamer::new::<G>(), io::stdin(), &mut io::stdout())`.
- `http_main` - reads `ADDR`, **defaults to `0.0.0.0:8080`** (e F46), parses to
  `SocketAddr`, awaits `brdgme_cmd::http::serve::<G>(addr)`.
- `fuzz_main` - non-async, no tokio, body is exactly
  `brdgme_fuzz::fuzz_gamer::<G>()` (kept in-process per D-43).

**Syntax, do not get this wrong:** declare `pub fn cli_main<G: ...>()`, **not**
`pub fn cli_main::<G>()`; turbofish is for call sites only. Bound on all three:
`G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static`, on each fn
or via one blanket-impl'd `pub trait GameBin: ...` - either, be consistent.
**Verify it against `requester::gamer::new`, `http::serve` and `fuzz_gamer`
first** (as read, `gamer::new` omits `'static` and `serve` requires it;
`fuzz_gamer`'s own bound is byte-for-byte the uniform bound above, so it fits;
confirm rather than trust this line).

`http_main` is a **non-async** fn owning the runtime: put `#[tokio::main]` on a
private `async fn http_main_inner<G>()` and call it - simpler than a hand-built
`Builder`, and what the game bins do today. That is the point of the change:
`#[tokio::main]` and the tokio dep leave 27 crates for this one.

`Cargo.toml`: inherit `[workspace.package]` and `[lints]` (post-WP-64). Deps:
`brdgme_cmd = { path = "../cmd" }` (default features - `http-server` is
default-on and gates `http`), `brdgme_game = { path = "../game" }`, `serde`, and
`tokio = { features = ["macros", "rt-multi-thread"] }` - the minimum for
`#[tokio::main]` - **plus `brdgme_fuzz = { path = "../../tools/fuzz" }`**, which
`fuzz_main` calls through. **Do not write `features = ["full"]`;** if it will not
compile, widen minimally and record which feature was needed.

**Stated crate-graph cost, do not hide it:** `brdgme_game_bin` -> `brdgme_fuzz`
-> `brdgme_cmd`, `brdgme_rand_bot`, `num_cpus`, `rand`, `anyhow`, so `_cli` and
`_http` now transitively link the fuzz tool's subtree. **Judged acceptable**:
`brdgme_cmd`/`rand` are already there, and the additions (`brdgme_rand_bot` plus
its `chrono`/`serde_json`, `num_cpus`, `anyhow`) are small; the cost is compile
time and image size, not runtime surface. If image size regresses measurably,
gate the dep behind a `fuzz` cargo feature.

### 3c. Wrapper bins - concrete after, `tic-tac-toe-2`

`src/bin/tic_tac_toe_2_http.rs` is 13 lines today (`#[tokio::main] async fn
main()`, `env::var("ADDR")`, parse, `serve::<Game>(addr).await`). After - same
path, same file name:

```rust
fn main() {
    brdgme_game_bin::http_main::<tic_tac_toe_2::Game>();
}
```

`tic_tac_toe_2_cli.rs` collapses identically to `cli_main`, and
`tic_tac_toe_2_fuzz.rs` (today `brdgme_fuzz::fuzz_gamer::<Game>();`) to
`fuzz_main`. All three end up 3 lines.

### 3d. D-41 + D-43: delete the 27 `_repl` bins ONLY

Delete `rust/game/*/src/bin/<snake>_repl.rs` (27). **Do NOT delete the 27
`<snake>_fuzz.rs` bins** - D-43 reverses the fuzz half of D-41. Bins are
auto-discovered from `src/bin/`, so deleting the files suffices - no game
manifest has a `[[bin]]` stanza to clean up.

**Why (`planning/fuzz-throughput-evaluation.md`, Lead-ACCEPTED):** the criterion
for fuzzing is raw throughput, and out-of-process fuzz via `LocalRequester`
costs a process spawn **per move** plus a second full JSON layer over the state
payload - directionally strictly slower, so rejected. `fuzz_main::<G>()` is
speed-neutral: same monomorphised `fuzz_gamer::<G>` call, only the `main` moves
crate. `_repl` is interactive with no throughput need, so D-41 stands there.

**Evidence: `planning/raw/wp73-fuzz-repl-dependency-sweep.md`, verdict CLEAN.**
It grepped the whole repo - `rust/Dockerfile`, `docker-bake.hcl`, `Tiltfile`,
`k8s/`, `scripts/`, `.github/`, `infra/`, `devenv.nix`, ignore files, all Rust
and Go tests, `brdgme-go` - for `_fuzz`, `_repl`, `fuzz_gamer`, `brdgme_fuzz`,
`brdgme_repl`; no justfile/Makefile exists. Nothing builds, deploys, tests or
shells out to the per-game `_repl` binaries. `rust/tools/repl` replaces them:
`cargo run -p brdgme_repl -- local target/release/<snake>_cli`. (The sweep's
fuzz half - "delete `fuzz_gamer`", "accepted slowdown" - is SUPERSEDED by D-43;
read it for the `_repl` evidence only.)

Follow-ons, all in scope here:

- Remove `brdgme_fuzz = { path = "../../tools/fuzz" }` from all 27 game
  manifests (present in exactly 27 of 27) - folded into 3e's edit. **Not because
  it became unused**: the `_fuzz` bins reach it one level up now, through
  `brdgme_game_bin`'s own `brdgme_fuzz` dep (3b).
- **`fuzz_gamer` is NOT deleted** and `rust/tools/fuzz/src/lib.rs` is not touched
  at all; it gains one caller (`fuzz_main`) and loses 27. So
  `specs/WP-63-fuzz-tool.md`'s `bo F29` reasoning about the `*_fuzz` bins going
  through `fuzz_gamer` stays TRUE - no WP-63 rewrite, no file overlap with it.
- Update `docs/porting/GAME_PORTING.md` (approximate lines 63, 74-75, 214,
  verify): **keep** the `<name>_N_fuzz.rs` layout entry and the
  `cargo run --bin <name>_N_fuzz` instruction; **remove only** the
  `<name>_N_repl.rs` entry and any repl invocation, replacing repl with
  `cargo run -p brdgme_repl -- local target/release/<snake>_cli`. Its
  `Cargo.toml` deps line drops `brdgme_fuzz` and gains `brdgme_game_bin`
  (matching 3e); "four ~12-line stubs" becomes three 3-line stubs.

### 3e. Manifest edits - targeted, not a canonical block

Per game crate: remove the three lines `brdgme_cmd`, `brdgme_fuzz`, `tokio` from
`[dependencies]`; add `brdgme_game_bin = { path = "../../lib/game_bin" }`.
Nothing else. `tic-tac-toe-2` `[dependencies]` becomes exactly
`brdgme_game_bin`, `brdgme_color`, `brdgme_game`, `brdgme_markup`,
`rand = "0.10.2"`, `serde = { version = "1.0.228", features = ["derive"] }`.

**`[dev-dependencies]` MUST STAY UNCHANGED**, in particular
`brdgme_cmd = { path = "../../lib/cmd", features = ["test-support"] }` - every
crate has `tests/contract.rs` calling
`brdgme_cmd::test_support::assert_gamer_contract::<Game>()`.

### 3f. Workspace

`rust/Cargo.toml` `[workspace] members` is an explicit list with no globs: add
one line, `"lib/game_bin"`. Because the wrappers stay as `src/bin/` files inside
the existing game crates, **no new per-game members** are needed and **no
Dockerfile, docker-bake.hcl, Tiltfile or k8s change is required.**

## 4. Per-game divergence

Essentially none - all bins are semantically identical (normalised checksum;
only rustfmt import *order* varies). The real divergences are library deps,
untouched here: `acquire-1` adds `thiserror`; `lords-of-vegas-1` adds
`thiserror` + `lazy_static`; `seven-wonders-1` adds `brdgme_cost`; `cathedral-2`
has no `rand`. Each manifest edit is a targeted removal of three named lines
plus one addition - **not** a canonical block copy-pasted over all 27.

**D-42:** `lords-of-vegas-1` gets the consolidation and the 3d deletion like
every other game crate, but is **not deployed** (no Dockerfile stage, bake
target, Tiltfile entry or k8s dir) and that is deliberate - **do not add it to
any of them.** Only the bins change.

## 5. Non-goals

- Any macro; any bin-target rename; any `[[bin]]` stanza.
- WP-71's warp -> axum rewrite, dep version pins, and the extra lib deps in 4.
- **Fuzz throughput work.** `planning/fuzz-throughput-evaluation.md` 4(d) (drive
  `Gamer` directly, skipping the API/render/serde layer in the hot loop) is OUT
  OF SCOPE and awaits Michael's decision on the render-coverage tradeoff. WP-73
  is speed-neutral by construction; do not smuggle a hot-loop change into it.

## 6. Landing order

- **After WP-64 (workspace tables)**, per `landing-order.md` 8.4: WP-64 adds
  `[workspace.dependencies]`, so each game manifest is edited once, and
  `game_bin` can inherit `[workspace.package]`/`[lints]`, which exist only
  post-WP-64.
- **WP-71** rewrites `lib/cmd/src/http.rs::serve`, which `http_main` calls. No
  hard dependency, but if both are pending land WP-71 first; otherwise re-verify
  `serve`'s signature before writing `http_main`.
- **WP-63 (fuzz tool): no file overlap** - post-D-43 this package never edits
  `rust/tools/fuzz/src/lib.rs`, so that flagged conflict is gone; either order.
- **D-17 standing constraint** (WP-64..WP-73): for any dependency problem,
  upgrade everything to latest FIRST, then re-assess.
- Adding `lib/game_bin` makes the workspace **41 members**, staling
  `specs/WP-64-workspace-tables.md`'s "40 members" regression assertion - update
  it here. (`landing-order.md` 8.6 flags the same hazard for WP-66.)

## 7. Verification

Read-only, before and after:

- `grep -rn '\[\[bin\]\]' rust/game/*/Cargo.toml` -> zero hits, both times.
- `ls rust/game/*/src/bin/` -> **108 files before, 81 after**: exactly the 27
  `<snake>_cli.rs`, 27 `<snake>_http.rs` and 27 `<snake>_fuzz.rs`, names
  unchanged. Any difference beyond the 27 `_repl` removals is a bug.
- Every `<snake>_http` in `rust/Dockerfile`'s 26 `COPY --from=builder` lines
  still exists as a `src/bin/` file.
- `grep -rn 'brdgme_fuzz' rust/game/*/Cargo.toml` -> 27 before, **zero after**
  (the dep moves up); `rust/lib/game_bin/Cargo.toml` -> **1 hit after**.
- `grep -rn 'fuzz_gamer' rust/` -> **non-zero after**: the definition in
  `rust/tools/fuzz/src/lib.rs` plus one call in `rust/lib/game_bin/src/lib.rs`.
  Zero hits means the fuzz path was deleted in error.
- `ls rust/game/*/src/bin/*_repl.rs` -> zero hits after.
- `grep -rn 'test-support' rust/game/*/Cargo.toml` -> 27 hits, unchanged.
- `grep -rn '0.0.0.0:80"' rust/` -> zero hits after.

When implementing (legitimate builds): pilot `cargo build -p brdgme_game_bin`
then `-p tic-tac-toe-2` before touching the other 26; then per-crate
`cargo clippy -p <crate> --all-targets` / `cargo test -p <crate>`.
`rust/tools/fuzz` is not edited, so no `-p brdgme_fuzz` check is needed.
AGENTS.md forbids workspace-wide builds on dev machines; CI's
`--workspace --exclude web --all-targets` covers all 81 wrappers. Confirm
`cargo build --release -p tic-tac-toe-2` still produces
`target/release/tic_tac_toe_2_http` (what the Dockerfile copies),
`tic_tac_toe_2_fuzz` (still runnable in-process) and
`tic_tac_toe_2_cli` (what the generic tools take as a path).
