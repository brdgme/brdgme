# WP-73 raw inventory: per-game bin targets (read-only research)

Method: reading only. No cargo/build/git commands. Live `master` working tree
(post-snapshot; `f8763a5` is stale). All 27 crates' bin files were compared
mechanically (md5 after normalising away the crate name); all four bin files
of 7 crates (acquire-1, red7-1, seven-wonders-1, starship-catan-1, splendor-2,
lost-cities-2, tic-tac-toe-2) were read in full.

## 1. Shape of the per-game bin targets

Counts:
- 27 game crates under `rust/game/`, each with exactly 4 files in `src/bin/`.
- 108 bin files total. **Zero deviations**: no crate is missing a bin, no crate
  has an extra bin, naming is uniformly `<crate_snake>_{cli,fuzz,http,repl}.rs`.
- Verified by normalised-md5 bucketing: cli 27/27 identical, fuzz 27/27,
  http 27/27, repl 27/27, modulo the crate name in `use <crate>::Game;`.
  The ONLY textual variation is `rustfmt` import ordering: `use <crate>::Game;`
  sorts before or after the `brdgme_cmd` imports depending on the crate name
  (e.g. `acquire_1` sorts first, `red7_1`/`splendor_2` sort last). No semantic
  divergence whatsoever.
- File sizes: cli 13 ln, http 13 ln, repl 8 ln, fuzz 5 ln (~39 ln/crate).

Representative literals (verbatim, `tic-tac-toe-2`):

`src/bin/tic_tac_toe_2_cli.rs`
```rust
use std::io;

use brdgme_cmd::cli::cli;
use brdgme_cmd::requester;
use tic_tac_toe_2::Game;

fn main() {
    cli(
        &mut requester::gamer::new::<Game>(),
        io::stdin(),
        &mut io::stdout(),
    );
}
```

`src/bin/tic_tac_toe_2_fuzz.rs`
```rust
use tic_tac_toe_2::Game;

fn main() {
    brdgme_fuzz::fuzz_gamer::<Game>();
}
```

`src/bin/tic_tac_toe_2_http.rs`
```rust
use std::{env, net::SocketAddr};

use brdgme_cmd::http;
use tic_tac_toe_2::Game;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = env::var("ADDR")
        .unwrap_or("0.0.0.0:80".to_string())
        .parse()
        .expect("Invalid socket address");
    http::serve::<Game>(addr).await
}
```

`src/bin/tic_tac_toe_2_repl.rs`
```rust
use brdgme_cmd::repl;
use brdgme_cmd::requester;
use tic_tac_toe_2::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
```

Addr/port handling is byte-identical in all 27 http bins: `ADDR` env var,
default `0.0.0.0:80`, `#[tokio::main]` (multi-thread default flavour).

## 2. The functions being called (`rust/lib/cmd/`)

Live signatures (read from source):
- `rust/lib/cmd/src/cli.rs`:
  `pub fn cli<R: Requester, I: Read, O: Write>(requester: &mut R, input: I, output: &mut O)`
  Not generic over `Gamer`; takes any `Requester`.
- `rust/lib/cmd/src/repl.rs`:
  `pub fn repl<T>(client: &mut T) where T: Requester` (re-exported as
  `brdgme_cmd::repl` from `lib.rs`).
- `rust/lib/cmd/src/http.rs`:
  `pub async fn serve<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>(addr: impl Into<SocketAddr>)`
  The only entry point with a `Gamer` bound. Internals: `env_logger::init()`,
  optional sentry init from `SENTRY_DSN_SERVER`/`SENTRY_RELEASE`, warp POST
  route with 16 MiB content-length limit, SIGTERM graceful shutdown.
- `rust/lib/cmd/src/requester/gamer.rs`:
  `pub fn new<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> GameRequester<G>`
  This is where the `Gamer` bound enters for cli/repl/fuzz.
- `rust/tools/fuzz/src/lib.rs`:
  `pub fn fuzz_gamer<G>() where G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static`
  and `pub fn fuzz<F, R>(new_requester: F) where F: Fn() -> R + Send + 'static, R: requester::Requester + 'static`.

So the canonical bound for a generic bin crate is:
`G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static`.

Feature gating (`rust/lib/cmd/Cargo.toml`, verified):
```
[features]
default = ["http-server"]
http-server = ["warp", "tokio", "sentry"]
test-support = []
```
`lib.rs` has `#[cfg(feature = "http-server")] pub mod http;` and
`#[cfg(feature = "test-support")] pub mod test_support;`.
`warp = { version = "0.4.3", features = ["server"], optional = true }`,
`tokio = { version = "1", features = ["signal"], optional = true }`,
`sentry = { version = "0.48", optional = true }`. Non-optional deps include
`term_size 0.3.2`, `env_logger 0.11.11`, `rand 0.10.2`, `time 0.3`.

## 3. Per-game `Cargo.toml`

- **There are NO `[[bin]]` stanzas anywhere.** All 27 manifests rely on cargo's
  auto-discovery of `src/bin/*.rs`. Grep for `[[bin]]` across `rust/game/*/Cargo.toml`
  returns 0 in every file. This matters: a generic bin crate approach must either
  keep `src/bin/` files or add explicit `[[bin]] name=... path=...` stanzas.
- Uniform `[dependencies]` block in all 27 (order and text identical):
  `brdgme_cmd`, `brdgme_fuzz`, `brdgme_color`, `brdgme_game`, `brdgme_markup`,
  `rand = "0.10.2"`, `serde = { version = "1.0.228", features = ["derive"] }`,
  `tokio = { version = "1.52.3", features = ["full"] }`.
  tokio and serde declarations are byte-identical across all 27.
- Deps existing ONLY to serve the bins: `brdgme_cmd` (cli/repl/http),
  `brdgme_fuzz` (fuzz), `tokio` "full" (http). `brdgme_color`/`brdgme_markup`/
  `brdgme_game`/`rand`/`serde` are genuine lib deps.
- Divergences (only 4 crates deviate, all additive lib deps):
  - `acquire-1`: extra `thiserror = "2.0.18"`.
  - `lords-of-vegas-1`: extra `thiserror` and `lazy_static`.
  - `seven-wonders-1`: extra `brdgme_cost = { path = "../../lib/cost" }`.
  - `cathedral-2`: the only crate with NO `rand` dep.
- `[dev-dependencies]`: all 27 have
  `brdgme_cmd = { path = "../../lib/cmd", features = ["test-support"] }`;
  12 additionally have `serde_json` (mixed pins: `"1.0.150"` in 8, `"1.0"` in 4).
- Every crate has `tests/contract.rs` calling
  `brdgme_cmd::test_support::assert_gamer_contract::<Game>()`. Any restructuring
  must keep the `test-support` dev-dep wired.
- No in-repo crate depends on a game crate as a library (grep for
  `path = "../../game` / `../game` across all manifests: zero hits). The
  "transitive build cost for library consumers" argument is currently vacuous;
  the real cost is 27x tokio-"full" compiles in the workspace build.

## 4. How the fuzz bins work

`rust/tools/fuzz` (package `brdgme_fuzz`) is a **hand-rolled random-command
fuzzer**, NOT afl/libfuzzer/cargo-fuzz. No `arbitrary`, no `afl`, no
`libfuzzer-sys` anywhere in its manifest. Deps: `brdgme_cmd`, `brdgme_game`,
`brdgme_rand_bot`, `anyhow`, `num_cpus`, `rand`, `serde`.

Mechanics: `fuzz()` spawns `num_cpus::get()` threads, each with its own
`Requester`; a `Fuzzer` iterator creates games (`Request::New` with a random
`u64` seed), picks random active players, generates commands via
`brdgme_rand_bot::spec_to_command`, and reports a tally each second; on error
it prints the seed + command log and stops. `fuzz_gamer::<G>()` is just
`fuzz(requester::gamer::new::<G>)`.

Nothing here resists a generic wrapper - it is a plain generic function call.

**Notable:** `rust/tools/fuzz/src/main.rs` ALREADY is a generic fuzz binary:
```rust
fn main() {
    let args: Vec<String> = env::args().collect();
    brdgme_fuzz::fuzz(move || requester::parse_args(&args).unwrap());
}
```
`requester::parse_args` accepts `local <path>` and builds a `LocalRequester`
that shells out to a game's `_cli` binary over stdin/stdout JSON.
Likewise `rust/tools/repl` (package `brdgme_repl`, one dep: `brdgme_cmd`) is
already a generic subprocess-driven repl. So generic out-of-process variants of
fuzz and repl EXIST; the 27 per-game `_fuzz`/`_repl` bins are the in-process
convenience variants.

## 5. Workspace wiring

`rust/Cargo.toml` `[workspace] members` is an **explicit list, no globs** - 40
entries: `bot`, 27 `game/<name>` paths, 7 `lib/*`, 3 `tools/*`, `web`,
`operator`. `resolver = "2"`. The list is not alphabetically sorted
(`game/age-of-war-2` after `game/alhambra-1`, `game/modern-art-2` mid-list,
`game/seven-wonders-1` before `game/roll-through-the-ages-2`).

Consequences: `lib/game_bin` must be added explicitly to `members`. Any new
per-game wrapper crate would ALSO need an explicit member entry (27 more lines).
Keeping the bins inside the existing game crates avoids touching members at all
beyond the one `lib/game_bin` line. There are also custom profiles
(`dev`, `android-dev`, `server-dev`, `wasm-dev`, `wasm-release`).

## 6. Downstream consumers of the binaries

Only `_http` binaries are consumed by deployment. `_cli`/`_repl`/`_fuzz` have
NO consumer outside the repo's dev workflow (and `LocalRequester`, which takes
an arbitrary path argument at runtime).

- **`rust/Dockerfile`** - the critical consumer. Builder stage runs
  `cargo build --release --workspace --exclude web`, then **26** distroless
  game stages each doing:
  ```
  FROM gcr.io/distroless/cc-debian12@sha256:7ee0... AS <game-slug>
  COPY --from=builder /app/target/release/<game_snake>_http .
  USER 65532
  CMD ["./<game_snake>_http"]
  ```
  Exact binary names expected (26): `acquire_1_http`, `age_of_war_2_http`,
  `alhambra_1_http`, `battleship_2_http`, `category_5_2_http`,
  `cathedral_2_http`, `farkle_2_http`, `for_sale_2_http`, `greed_2_http`,
  `jaipur_2_http`, `liars_dice_2_http`, `lost_cities_1_http`,
  `lost_cities_2_http`, `love_letter_2_http`, `modern_art_2_http`,
  `no_thanks_2_http`, `red7_1_http`, `roll_through_the_ages_2_http`,
  `seven_wonders_1_http`, `splendor_2_http`, `starship_catan_1_http`,
  `sushi_go_2_http`, `sushizock_2_http`, `texas_holdem_2_http`,
  `tic_tac_toe_2_http`, `zombie_dice_2_http`.
  **Any rename of a `*_http` target breaks the image build.** The `COPY` is by
  flat filename from `target/release/`, so target names must stay globally
  unique across the workspace.
- **`lords-of-vegas-1` has NO Dockerfile stage, no bake target, no Tiltfile
  entry, no `k8s/base/game/` directory.** It is a workspace member with all 4
  bins but is not deployed. 27 crates, 26 deployed.
- **`docker-bake.hcl`** - matrix of 30 targets (`web`, `migrate`, `bot`,
  `operator` + the 26 game slugs). Uses the Dockerfile stage names
  (hyphenated crate names), not binary names.
- **`Tiltfile`** (line ~21) - hardcoded list of the same 26 game slugs,
  `docker_build("brdgme/" + game, ".", dockerfile="rust/Dockerfile", target=game, only=["rust/"])`.
- **`k8s/base/game/<slug>/`** - 44 directories (includes retired `-1` versions
  with no Rust crate). Each has `deployment.yaml` (`image: brdgme/<slug>`,
  `env: ADDR=0.0.0.0:8080`, containerPort 8080, tcp readiness/liveness probes),
  `service.yaml`, `http-scaled-object.yaml` (KEDA), `game-version.yaml`
  (CRD `brdgme.com/v1 GameVersion` with typeName/weight/blurb/interfaceVersion).
  These reference the **image** name, never the binary name.
- **`game_versions.uri`** is written by `rust/operator/src/controller.rs`
  (`interceptor_uri()`, upserted via SQL into `game_versions`) and points at the
  KEDA interceptor proxy - derived from k8s service names, NOT binary names.
  Renaming a Rust bin does not touch it.
- **CI** (`.github/workflows/ci.yml`) uses `docker/bake-action` with
  `docker-bake.hcl`; `scripts/rust-ci-commands.sh` runs
  `cargo clippy --workspace --exclude web --all-targets` and
  `cargo test --workspace --exclude web`. `--all-targets` compiles every bin,
  so all 108 bins are clippy-linted today.
- No shell script, Makefile, or other manifest references `_cli`/`_repl`/`_fuzz`
  binary names.

## 7. The four findings in scope

**`dp F11`** = `findings/dependencies.md` "Game-crate tokio uses features =
["full"]; feature-set drift on shared deps" (severity minor). Quote: "All 27
game crates declare `tokio = { version = "1.52.3", features = ["full"] }`, but
their only async surface is the 13-line `*_http` bin calling
`brdgme_cmd::http::serve`". **CORRECT** against live code - verified identical
in all 27 manifests, and the http bin is exactly 13 lines. Its recommendation
("reduce to `rt-multi-thread`, `macros`, or let a shared bin crate own the
tokio dep") is sound; note `lib/cmd` itself already pulls `tokio` with the
`signal` feature, which `http::serve` requires for SIGTERM shutdown.

**`dp F26`** = `findings/dependencies.md` "27 game crates x 4 boilerplate
binaries = 108 near-identical files" (severity minor, category simplicity).
Quote: "Every game crate ships `_cli`/`_repl`/`_fuzz`/`_http` bins (5-13 lines
each, ~38 lines/crate, 108 files total) whose only variation is the crate name
in `use <game>::Game;`". **CORRECT** - counts and the "only variation" claim
both verified exactly (39 ln/crate by my count; 5/8/13/13). Its recommendation
proposes a `brdgme_game_bins!(Game)` macro OR "one generic bin crate
parameterised by feature/env"; D-20 option B selects the latter, consistent
with the repo's macro-caution stance.

**`e F45`** = `findings/games-batch-e.md` "Binary-only dependencies declared as
library `[dependencies]`". Quote: "`brdgme_cmd`, `brdgme_fuzz`, and
`tokio = { features = ["full"] }` ... are only used by the `src/bin/` binaries
... Binaries in the same package can use `[dev-dependencies]`, so there is no
need for these to be lib deps." Recommendation: move them to
`[dev-dependencies]`.
**Facts CORRECT; recommendation INVALID - confirmed.** Cargo does not link
`[dev-dependencies]` into `src/bin/` targets (dev-deps apply to tests, examples
and benches only), so the move would break all 108 bins. `findings/verification/
games-batch-e.md` already records this as ADJUSTED with the same reasoning and
notes the correct alternatives (optional deps + `required-features` on an
explicit `[[bin]]`, or a separate bin crate). It also notes no in-repo crate
consumes a game crate as a library, which I independently confirmed - so the
transitive-cost argument is vacuous and the realizable win is the tokio trim.

**`e F46`** = `findings/games-batch-e.md` "HTTP binary defaults to privileged
port 80" (severity nit). Quote: "`env::var("ADDR").unwrap_or("0.0.0.0:80".to_string())`
- binding port 80 requires root / CAP_NET_BIND_SERVICE, so the binary fails out
of the box when run unprivileged without `ADDR` set". **CORRECT** and the
consequence is sharper than the finding states: the distroless runtime stages
set `USER 65532`, so the default is unusable in the shipped image too - it only
works because `k8s/base/game/*/deployment.yaml` sets `ADDR=0.0.0.0:8080`.
Changing the default to `0.0.0.0:8080` is safe: no consumer relies on port 80
(k8s sets ADDR explicitly, containerPort and both probes are 8080).

## Explicitly NOT checked

- Did not run cargo/clippy/build/test; nothing was compiled or verified by
  the toolchain.
- Did not read all 27 crates' bin files by eye - 7 crates read in full, the
  other 20 compared by normalised checksum only.
- Did not read `k8s/base/game/*/http-scaled-object.yaml` or `service.yaml`
  contents (only `acquire-1`'s deployment/game-version).
- Did not read `rust/bot`, `rust/web`, or `rust/operator` beyond grepping for
  `uri` and game-crate path deps.
- Did not check the Go side (`brdgme-go/`) - it has its own Dockerfile and
  game list; unaffected by Rust bin restructuring as far as I can tell, but not
  audited.
- Did not verify `Cargo.lock` contents.
