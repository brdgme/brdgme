# Verification: lib-support.md F19-F30 (lib/cmd)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5. All paths relative to that dir.

## F19 (major, correctness, lib/cmd/src/http.rs:54) - CONFIRMED

Evidence:
- http.rs:53-54:
  ```rust
  let mut g: GameRequester<G> = requester::gamer::new();
  let reply = warp::reply::json(&g.request(&req).unwrap());
  ```
- Reachability: `GameRequester::request` returns `Err(RequestError::Parse)` when the
  `game` string in the request body is not valid JSON for `G` - requester/gamer.rs:28
  (`Status`), :37 (`Play`), :41 (`PubRender`), :45 (`PlayerRender`) all do
  `serde_json::from_str(game)?`. `RequestError` has `#[from] serde_json::Error`
  (requester/error.rs:8-11).
- The `.unwrap()` runs inside the warp `.map` handler closure (http.rs:39-57), so an
  `Err` panics the connection task; `transaction.finish()` at :55 is skipped.
- http.rs:17 `impl Reject for RequestError {}` - grep for `Reject` in lib/cmd/src
  finds only the import (http.rs:7) and this impl; never used in a rejection path.
- Every game service binary uses this: e.g. game/acquire-1/src/bin/acquire_1_http.rs
  and 9+ sibling `*_http.rs` binaries call `http::serve`.

Reasoning: user-supplied malformed `game` JSON in an otherwise well-formed Request
body deserializes fine as `Request` (game is `String`, api.rs:18) and then panics in
the handler. Client gets a dropped/aborted connection instead of a SystemError
response. Major correctness defect in production path as stated.

## F20 (minor, correctness, lib/cmd/src/repl.rs:116) - CONFIRMED

Evidence:
- `:load` (repl.rs:112-115) and `:undo` (repl.rs:116-128) assign `game` only;
  `public_render`/`player_renders` are untouched (only updated on successful Play,
  repl.rs:154-155, and at New, repl.rs:38-51).
- Next loop iteration renders from `player_renders[current_player].render`
  (repl.rs:91) and `command_spec` (repl.rs:93) while a subsequent Play sends
  `game.state` (repl.rs:135).

Reasoning: after :undo/:load, display shows pre-undo/pre-load state while the engine
plays from the restored state - divergence until the next successful play. Confirmed.

## F21 (minor, simplicity, lib/cmd/src/bot_cli.rs:20) - CONFIRMED

Evidence:
- Workspace-wide grep for `bot_cli`: only lib/cmd/src/lib.rs:8 (mod decl) and
  lib/rand_bot/src/lib.rs:5,107. rand_bot uses only `bot_cli::Request`
  (lib/rand_bot/src/lib.rs:107) inside its own `cli` fn (lib.rs:102-114).
- No caller of `bot_cli::cli` or user of `bot_cli::Response` anywhere in the
  workspace (grep for `bot_cli::cli` and `bot_cli::Response`: zero hits).
- bot_cli.rs:29-43: `cli` contains four unwraps (:29, :30, :41, :43 - finding says
  "three", close enough; the count is not load-bearing).

Reasoning: `cli` fn and `Response` type alias (bot_cli.rs:20) are dead code beyond
the `Request` struct. Confirmed.

## F22 (minor, correctness, lib/cmd/src/repl.rs:226) - CONFIRMED

Evidence:
- repl.rs:232-234:
  ```rust
  let mut input = String::new();
  stdin().read_line(&mut input).unwrap();
  input.trim().to_owned()
  ```
  Byte count (Ok(0) at EOF) discarded; returns "".
- In the main loop (Status::Active), "" falls to the `_ =>` arm (repl.rs:130) and
  fires `Request::Play` with empty command; a UserError loops straight back to
  `prompt` (repl.rs:98) - infinite hot spin at EOF. (In the initial player-name loop
  empty input breaks, repl.rs:25-27, so only the main loop spins.)
- read_line error is `.unwrap()`ed at repl.rs:233 as cited.

Reasoning: EOF handling missing; confirmed as described. Dev-tool severity minor.

## F23 (minor, consistency, lib/cmd/src/requester/gamer.rs:67) - CONFIRMED

Evidence per sub-claim:
- cli.rs:10-18: request parse failure -> `Response::SystemError`, but
  `requester.request(&r).unwrap()` (:14) and the output writes `.unwrap()` (:16,:18).
- gamer.rs: `GameResponse::from_gamer` routes state-serialization errors through
  `GameResponseError` (api.rs:183-189) in handle_new/handle_status/handle_play, yet
  `renders` unwraps `serde_json::to_string(&pub_state)` (gamer.rs:67) and
  `&player_state` (:74), and handle_pub_render/handle_player_render unwrap the same
  serializations (:164, :177).
- repl.rs:177 (`brdgme_markup::from_string(&l.content).unwrap()`) and :219
  (`from_string(markup).unwrap()`) unwrap markup parses of log/render content.
- repl.rs:55: `_ => panic!("wrong reponse"),` - typo present.

Reasoning: all cited inconsistencies exist verbatim. Confirmed.

## F24 (minor, dependencies, lib/cmd/Cargo.toml:16) - CONFIRMED

Evidence:
- Cargo.toml:16: `term_size = "0.3.2"`.
- Single call site: repl.rs:186 `term_size::dimensions()` (workspace grep confirms
  no other use). RUSTSEC-2020-0163 advisory status taken as given per instructions.

## F25 (minor, consistency, lib/cmd/Cargo.toml:17) - CONFIRMED

Evidence:
- lib/cmd/Cargo.toml:17-18: `warp = { version = "0.4.3", ... }`, tokio with
  `signal` feature; both under `http-server` feature which is default (:24-25).
- web/Cargo.toml:20: `axum = { version = "0.8.9", ... }`;
  lib/game_client/Cargo.toml:23-24: axum 0.8.9 in `[dev-dependencies]`.
- Handler: http.rs:36-57 is ~22 lines; whole serve fn ~50 lines. "~30 lines" fair.
- Game service binaries (game/*/src/bin/*_http.rs, 10+ found) each link warp+tokio
  signal solely via `http::serve`.

Reasoning: two HTTP server stacks in one workspace as stated. Confirmed.

## F26 (nit, consistency, lib/cmd/src/repl.rs:147) - CONFIRMED

Evidence: repl.rs:147: `if remaining_input.trim() != "" {`. `!...is_empty()` is the
idiomatic form.

## F27 (nit, quality, lib/cmd/src/api.rs:14) - CONFIRMED

Evidence: api.rs:14-15:
```rust
#[serde(default)]
seed: Option<u64>,
```
serde_derive already treats missing `Option<T>` fields as `None` without
`#[serde(default)]` (documented serde behavior). Redundant attribute. Confirmed.

## F28 (nit, quality, lib/cmd/src/http.rs:38) - CONFIRMED

Evidence: http.rs:38: `.and(warp::body::json())` with no
`warp::body::content_length_limit` anywhere in the filter chain (http.rs:36-57).
warp's body filters impose no size cap unless one is added; game state strings in
Request bodies are caller-influenced in size. In-cluster exposure only, so nit
severity is right. Confirmed.

## F29 (nit, quality, lib/cmd/src/requester/local.rs:35) - CONFIRMED

Evidence: local.rs:35-41: `wait_with_output()` result's exit status is never
checked; stderr is relayed (:37-39) then `serde_json::from_slice(&output.stdout)?`
(:41). A crashed child with empty stdout surfaces as
`RequestError::Parse` ("failed to parse request", error.rs:7) rather than an
exit-status error. Confirmed.

## F30 (nit, correctness, lib/cmd/src/repl.rs:59) - CONFIRMED

Evidence: repl.rs:59: `let mut undo_stack: Vec<GameResponse> = vec![game.clone()];`
and successful plays push the pre-play game (repl.rs:152). With the seed entry,
`:undo` before any play pops the initial game and assigns it (repl.rs:116-118) - a
silent no-op reset - instead of hitting the "No undos available" branch
(repl.rs:119-127). More generally there is always one extra no-op undo before the
message appears. Confirmed.

# Summary

| # | Verdict | Severity |
|---|---------|----------|
| F19 | CONFIRMED | major |
| F20 | CONFIRMED | minor |
| F21 | CONFIRMED | minor |
| F22 | CONFIRMED | minor |
| F23 | CONFIRMED | minor |
| F24 | CONFIRMED | minor |
| F25 | CONFIRMED | minor |
| F26 | CONFIRMED | nit |
| F27 | CONFIRMED | nit |
| F28 | CONFIRMED | nit |
| F29 | CONFIRMED | nit |
| F30 | CONFIRMED | nit |
