# WP-73 dependency sweep: deleting the per-game `_fuzz` / `_repl` binaries

## Verdict: CLEAN - safe to delete

No build, deploy, CI, container, k8s, test-harness or Go-side dependency on any
per-game `_fuzz` or `_repl` binary exists. The only references outside the 54 bin
files themselves are:

- `brdgme_fuzz` dependency lines in all 27 game `Cargo.toml` files (become unused).
- One documentation reference: `docs/porting/GAME_PORTING.md` (see Consequences).

## Premise confirmation

Confirmed. Both tools are generic out-of-process drivers.

- `rust/tools/repl/src/main.rs` - entire body: collects `env::args()`, calls
  `requester::parse_args(&args).unwrap()`, then `repl(&mut client)`.
- `rust/tools/fuzz/src/main.rs` - entire body: `brdgme_fuzz::fuzz(move || requester::parse_args(&args).unwrap())`.
- `rust/lib/cmd/src/requester/mod.rs` -
  `pub fn parse_args(args: &[String]) -> Result<impl Requester + use<>, ParseArgsError>`.
  Accepts exactly one mode: `"local" <path>` -> `local::LocalRequester::new(&args[2])`.
- `rust/lib/cmd/src/requester/local.rs` - `LocalRequester { path: OsString }`; each
  `request()` spawns `Command::new(&self.path)`, writes the JSON `Request` to stdin,
  reads the JSON `Response` from stdout. Path is runtime data; nothing is hardcoded.
- `rust/lib/cmd/src/cli.rs` - `pub fn cli<R: Requester, I: Read, O: Write>(...)` reads one
  `Request` from stdin, writes one `Response`. This is what every `<snake>_cli` bin runs
  (e.g. `rust/game/tic-tac-toe-2/src/bin/tic_tac_toe_2_cli.rs`).

So `cargo run -p brdgme_fuzz -- local target/release/<snake>_cli` (and the same for
`brdgme_repl`) fully replaces the per-game bins. Neither tool names a `_fuzz`/`_repl`
binary anywhere.

`fuzz_gamer` reference check: `rust/tools/fuzz/src/lib.rs` defines
`pub fn fuzz_gamer<G>()` (approximate line 92, verify) which calls
`fuzz(requester::gamer::new::<G>)`. Its ONLY callers repo-wide are the 27
`src/bin/*_fuzz.rs` files. After deletion `fuzz_gamer` is dead and should be deleted
too; `fuzz()` itself stays (used by `tools/fuzz/src/main.rs`).

## Sweep - commands and results

All run from repo root.

| Search | Result |
| --- | --- |
| `rg -n --hidden -g '!.git' -e '_fuzz' -e '_repl' -e 'fuzz_gamer' -e 'brdgme_fuzz' -e 'brdgme_repl' -l` | Only: docs (review/porting/superpowers plans), `rust/Cargo.lock`, the 54 bin files, 27 game `Cargo.toml`, `rust/tools/{fuzz,repl}` themselves. |
| Same grep with `-g '!docs/**' -g '!rust/Cargo.lock' -g '!rust/game/*/src/bin/*'` | Only the 27 `brdgme_fuzz = { path = "../../tools/fuzz" }` manifest lines plus the two tool manifests/sources. Every other `_repl` hit is a false positive on `replace` / `replay` / `replicas` / `reply` (`rust/web/**`, `rust/bot/src/prompt.rs`, game `src/lib.rs` test names). |
| `rg -n -i -e 'fuzz' -e 'repl' rust/Dockerfile docker-bake.hcl Tiltfile AGENTS.md README.md` | Zero fuzz/repl hits. Only `README.md:18` "email replies". `rust/Dockerfile` copies only `target/release/<snake>_http` per game stage (plus `web`, `bot`, `operator`, `sqlx`). |
| `rg -rn -i 'fuzz' k8s scripts .github infra devenv.nix` | No hits. (`.github` contains no workflow referencing these targets.) |
| `rg -n -i 'fuzz' brdgme-go` | No hits. Go side never shells out to these. |
| `find . -iname justfile -o -iname Makefile -o -iname '*.mk'` | None exist in the repo. |
| `rg -n -i 'fuzz\|repl' .gitignore .dockerignore` | No hits. |
| `rg -n -i '_repl\|cargo run\|--bin' Tiltfile docker-bake.hcl AGENTS.md docs/CODING.md scripts .github k8s infra devenv.nix .envrc .coree.toml` | Only two `cargo run` lines in `Tiltfile` (bot, operator). No game bins. |
| `rg -n -i 'fuzz\|brdgme_repl' rust/game/*/tests rust/web/tests rust/web/end2end` | No hits. |
| `rg -n '\[\[bin\]\]' rust --glob Cargo.toml -l` | `operator`, `bot`, `web`, `tools/render_plain` only. Game bins are auto-discovered from `src/bin/`, so deleting the files is sufficient - no manifest `[[bin]]` stanzas to remove. |
| No `.cargo/`, `.vscode/`, `.devcontainer/` directories exist at repo root or under `rust/`. | n/a |

Game-crate `src/` usage of `brdgme_fuzz` outside `src/bin/`: none. Verified by the
second grep row above - every non-bin `brdgme_fuzz` hit is a `Cargo.toml` line.

## Consequences

- Delete 54 files: `rust/game/*/src/bin/<snake>_fuzz.rs` (27) and
  `rust/game/*/src/bin/<snake>_repl.rs` (27). All 27 fuzz bins are byte-identical modulo
  the crate name (`brdgme_fuzz::fuzz_gamer::<Game>();`); all 27 repl bins likewise
  (`repl(&mut requester::gamer::new::<Game>());`). Verified by diffing the bodies.
- Remove `brdgme_fuzz = { path = "../../tools/fuzz" }` from all 27
  `rust/game/*/Cargo.toml` (confirmed present in exactly 27 of 27). This is the only
  manifest dep that becomes unused. `brdgme_cmd` must STAY (used by the `_cli` and
  `_http` bins).
- Delete `fuzz_gamer` from `rust/tools/fuzz/src/lib.rs` (its only callers are removed).
  Keep `fuzz()`. `rust/lib/cmd/src/requester/gamer.rs` must STAY - still used by the
  `_cli`/`_http` bins, `rust/lib/cmd/src/http.rs` and `rust/lib/cmd/src/test_support.rs`.
- `brdgme_cmd`'s `repl` module stays; after deletion its only consumer is
  `rust/tools/repl`.
- Doc update required: `docs/porting/GAME_PORTING.md` lists `<name>_N_repl.rs` and
  `<name>_N_fuzz.rs` in the crate layout (approximate lines 63, 74-75, 214, verify) and
  instructs `cargo run --bin <name>_N_fuzz`. Replace with the generic-tool invocation.
- Behavioural cost (not a blocker): `LocalRequester` spawns one child process per API
  request, so out-of-process fuzzing is slower than the in-process `fuzz_gamer` path.
  Michael should be aware; it is the price of the consolidation.

## Explicitly NOT checked

- Nothing was built, run, or compiled. No `cargo check`. Correctness of the resulting
  workspace after deletion is inferred from source reading only.
- `rust/Cargo.lock` was not edited/analysed; it will regenerate.
- Michael's local shell history / aliases / untracked personal scripts.
- `rust/target/` build artifacts (stale `<snake>_fuzz` binaries may linger there).
- Other review specs that mention `_fuzz`/`_repl` in prose (e.g.
  `planning/specs/WP-63-fuzz-tool.md`, `planning/specs/WP-73-game-binary-consolidation.md`,
  `findings/dependencies.md`) - these are planning documents, not dependencies, and were
  not reconciled against this sweep.
- `docs/superpowers/plans/*` historical plans that mention the bins - historical records,
  left alone.
