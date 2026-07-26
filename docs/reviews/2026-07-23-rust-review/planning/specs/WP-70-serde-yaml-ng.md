# WP-70: `serde_yaml` -> `serde_yaml_ng`

**Findings:** dp F14, bo F17, ls F34 (all minor; ls F34 CONFIRMED in
`findings/verification/lib-support.md`) - three views of one problem.
**Decision:** D-21.

**Landing order:** after WP-64 (`serde_yaml` is in its hoist list, so this is a
one-line root edit). Independent of everything else. WP-69 lands last.

> **Read every named file/function before editing. No line numbers are cited on
> purpose; the tree is under concurrent edit. If a file does not match what this
> spec describes, STOP and report rather than improvising.**

## 0. Step 0 - upgrade to latest FIRST (binding, do not skip)

Michael's standing strategy is to stay as close to latest as possible so deps
never go stale: the first step for any dependency problem is **"upgrade all
dependencies to latest and see where we stand."** **Here it will not help -
`serde_yaml` was archived in 2024 at `0.9.34+deprecated` and there is no newer
version.** Confirm that once, then do the swap.

## 1. Problem

- **dp F14** - lock carries archived `serde_yaml 0.9.34+deprecated` and its
  archived backend `unsafe-libyaml 0.2.11`. Two direct consumers: `rust/bot`,
  `rust/lib/game_client`.
- **bo F17** - the bot's use, `spec_to_yaml` in `rust/bot/src/prompt.rs`.
- **ls F34** - `lib/game_client`'s use.

## 2. Why it's wrong

- All three are **correct as written**. Verified live: plain, non-optional
  `serde_yaml = "0.9"` in both manifests. Exactly two consumers.
- **bo F17's recommendation is half wrong** - it offers a fork *or* JSON. D-21
  rejected JSON: it would change a file format ops and users may depend on.
  **Do not "improve" this to JSON.**
- **Both consumers move together** (landing-order 8.2); migrating one leaves the
  archived crate in the tree via the other.

## 3. Required end state

Both call sites verified live, both **serialise-only** - no `from_str`, no
`Value`, no deserialisation anywhere in `rust/`. The only API to port is
`to_string`, which `serde_yaml_ng` exposes identically, so risk is near zero.

| Call site | API | Direction | Output goes to |
|---|---|---|---|
| `rust/bot/src/prompt.rs::spec_to_yaml` | `serde_yaml::to_string` on a `serde_json::Value` | serialise | an LLM prompt string; never leaves the process |
| `rust/lib/game_client/src/lib.rs::json_to_yaml` | `serde_yaml::to_string` on a `serde_json::Value` | serialise | `GameData.pub_state_yaml`/`player_state_yaml`, feeding that same prompt |

Grep `serde_yaml` across `rust/` first; if a third site or any deserialise call
exists, STOP and report. Both sites deliberately route through
`serde_json::Value` (mapping style vs YAML native enum tags - `prompt.rs`
documents why). **Keep that indirection and comment.**

Edits, all in one PR:

1. `rust/Cargo.toml` `[workspace.dependencies]`: `serde_yaml = "0.9"` ->
   `serde_yaml_ng = "<latest>"`. If WP-64 has not landed, make the same change
   in both crate manifests instead.
2. `rust/bot/Cargo.toml`, `rust/lib/game_client/Cargo.toml`: key renamed.
3. The two functions above: path becomes `serde_yaml_ng::to_string`. Nothing
   else in either function changes.

## 4. Non-goals

- JSON (rejected by D-21); `serde-yml` or any other fork (D-21 named
  `serde_yaml_ng`).
- Changing the YAML shape, the `Value` round-trip, or key order.
- `lib/game_client`'s other findings (ls F31-F37) - WP-07.

## 5. Regression test cases

- **Never run a bare workspace-wide `cargo build`/`test`/`clippy`** (AGENTS.md
  "Resource constraints"). Use `cargo check -p bot`,
  `cargo check -p brdgme_game_client`.
- **Byte-identical output is the acceptance criterion.** Before the swap,
  capture `spec_to_yaml` for a fixed `Spec` and `json_to_yaml` for a fixed JSON
  string (a scratch `#[test]` writing to a file). Repeat after, `diff`. If they
  differ, report it to the Lead rather than accepting it silently - the bot's
  system prompt documents the shape it expects.
- `serde_yaml` absent from `rust/Cargo.lock`. Check `unsafe-libyaml` too: if
  `serde_yaml_ng` still pulls the archived `0.2.11`, the backend half of dp F14
  is unresolved - say so in the PR rather than closing the finding.
- CI clippy split: `cargo clippy --workspace --exclude web --all-targets -- -D
  warnings`, then `cargo clippy -p web --all-targets --features ssr -- -D
  warnings`. Final gate:
  `/home/beefsack/Development/brdgme/scripts/rust-test.sh`.

## 6. Riders

| # | Item | Source |
|---|------|--------|
| 1 | Both manifests changed in one PR; neither crate left on `serde_yaml` | D-21, 8.2 |
| 2 | `prompt.rs`'s native-tags-vs-mapping comment retained, crate name updated | bo F17 |
| 3 | Byte-identical YAML confirmed for both functions, recorded in the PR | D-21 |
| 4 | `unsafe-libyaml` presence/absence after the swap recorded | dp F14 |
| 5 | Single hoisted entry; both members use `serde_yaml_ng.workspace = true` | WP-64 |
