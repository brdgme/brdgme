# 10: Eliminate runtime panics in rust/web - Design

> Extracted 2026-07-08 from `docs/plan/10-eliminate-runtime-panics.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Complete

**Goal:** Replace all panic-prone code in `rust/web/src` that could crash the
server process or the WASM frontend at runtime with proper error handling.
Startup panics (`main.rs`, `auth/session.rs`, `db.rs` env var) are
intentional and excluded.

**Background:** An audit of `rust/web/src` found 47 instances of `.unwrap()`,
`.expect()`, `unreachable!()`, and `panic!()`. Most are either in tests or in
`#[cfg(not(feature = "ssr"))]` stubs (correct). The runtime risks are
enumerated in the paired plan.

## Excluded (intentional)

- `main.rs`, `auth/session.rs`, `db.rs:55`: startup failures where the
  process cannot run without the resource. Panicking at boot is correct.
- `game/client.rs`: all within `#[cfg(test)]`. Panics in tests are fine.
- `server_fns.rs` `unreachable!()` in `#[cfg(not(feature = "ssr"))]` stubs:
  these paths are never compiled into the server and are never reachable on
  the client, so they are correct as-is.
- `components/game.rs:373` - `values.into_iter().next().unwrap()`: guarded by
  `values.len() == 1` on the preceding line; `unwrap()` is provably safe.
