# 1: Foundation & Shared Logic - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/01-foundation.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete

**Goal:** Make `brdgme_cmd` and `brdgme_game` compatible with WASM so they can
be used in the browser frontend.

- [x] Make `warp` an optional dependency in `rust/lib/cmd/Cargo.toml`.
- [x] Gate the `http` module in `rust/lib/cmd/src/lib.rs` behind
      `#[cfg(feature = "http-server")]`.
- [x] Verify WASM compilation: `cargo check --target wasm32-unknown-unknown -p brdgme_cmd`.
