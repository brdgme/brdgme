# 23: Rust Game Ports - Design

> Extracted 2026-07-08 from `docs/plan/23-rust-game-ports.md` (superpowers layout
> migration). Content dates from 2026-07-04; this is a point-in-time decision
> record, not a living document.

**Status:** Pending

## Background / decision

Decision to target Rust over Go (2026-07-04): `docs/GO_VS_RUST_PORTING.md`.
Method: `docs/GAME_PORTING.md`. Per-game analysis: `docs/GAME_PORTING_PLAN.md`.
Template: `rust/game/lost-cities-1`.

## Deferred (not ports - old versions incomplete)

- hive: old code is a stub (no commands, demo render); new Rust development
  incl. hex-grid rendering. (Partial Go bring-over in stash
  `wip-go-hive-chess-port`; superseded, do not build on it.)
- chess: old code is a move-generation engine only, never a playable game;
  game layer would be new development.
