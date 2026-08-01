# R-14 Context Handover - Share the NATS wire protocol

Survey date: 2026-08-01. HEAD: `77d5d0f47ed4ebb068954ce4a4e1482563d73f56`.

## Tracker references

- Plan: `docs/reviews/2026-07-30-review-session/98-REMEDIATION-PLAN.md:506-528`
- Progress: `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md:39` (R-14 pending, blocks R-15)
- Findings: F-108 (`90-findings-part2.md:33`), F-188 (`90-findings-part3.md:43`)
- Detailed analysis: `10-bot-operator-tools.md:98-132`

## Acceptance criteria (from 98-REMEDIATION-PLAN.md:520-527)

1. Wire types live in one crate; both `rust/bot` and `rust/web` depend on it.
2. Round-trip test serialises with one crate's types and deserialises with the
   other's - or, once shared, asserts wire format against a golden fixture.
3. Duplicated constants gone; grep confirms one definition each.

## Current duplicated files

| File | Lines | Role |
|------|-------|------|
| `rust/bot/src/nats.rs` | 35 | Bot-side: `BotTurnEvent`, `BotCommandEvent`, `STREAM_NAME`, `SUBJECT_COMMAND`, `CONSUMER_TURN`, `connect()` |
| `rust/web/src/nats.rs` | 548 | Web-side: same types + `SUBJECT_TURN`, `CONSUMER_COMMAND`, `MAX_TURN_ATTEMPTS`, `MAX_DELIVER`, `connect()`, `ensure_stream_and_consumers()`, drift detection, advisory listener, `supervise_consumer()`, tests |

## Wire types (field-identical today)

```rust
pub struct BotTurnEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub bot_name: String,
    pub attempt: i32,
}

pub struct BotCommandEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub command: String,
    pub attempt: i32,  // echoes BotTurnEvent::attempt
}
```

Both derive `Debug, Clone, Serialize, Deserialize`. Serde encoding is plain
JSON (no rename, no tag). `uuid` uses `serde` feature.

## Constants to share

| Constant | Value | Used by bot | Used by web |
|----------|-------|-------------|-------------|
| `STREAM_NAME` | `"BOT"` | yes (`main.rs:773`) | yes (`nats.rs:124`, `game/mod.rs:272`) |
| `SUBJECT_TURN` | `"bot.turn"` | no | yes (`game/mod.rs:236`) |
| `SUBJECT_COMMAND` | `"bot.command"` | yes (`main.rs:497`) | yes (`nats.rs:154`) |
| `CONSUMER_TURN` | `"bot-turn"` | yes (`main.rs:773`) | yes (`nats.rs:153`) |
| `CONSUMER_COMMAND` | `"bot-command"` | no | yes (`nats.rs:154`, `game/mod.rs:272`) |
| `MAX_TURN_ATTEMPTS` | `3i32` | no | yes (`game/mod.rs:468,511`) |
| `MAX_DELIVER` | `3i64` | no | yes (`nats.rs:162`, `game/mod.rs:397`) |

Additionally: `ACK_WAIT` (currently a local variable `Duration::from_secs(5*60)`
at `web/src/nats.rs:151`) should be promoted to a shared const per F-188's
concrete-harm analysis. The bot's `ACK_HEARTBEAT_INTERVAL` test
(`bot/src/main.rs:1038-1040`) hardcodes `5 * 60` - it must reference the shared
const instead.

## R-13 precedent (shared crate pattern)

R-13 created `rust/lib/crypto` (`brdgme_crypto`) as the model:

- `rust/lib/crypto/Cargo.toml`: workspace-inherited `version`, `edition`,
  `publish`, `authors`; `[lints] workspace = true`; deps from workspace where
  available.
- Consumers are thin re-export facades:
  - `rust/bot/src/crypto.rs:1` = `pub use brdgme_crypto::*;`
  - `rust/web/src/crypto.rs:1` = `pub use brdgme_crypto::*;`
- Bot depends unconditionally: `brdgme_crypto = { path = "../lib/crypto" }`
- Web depends optionally (ssr-gated): `brdgme_crypto = { path = "../lib/crypto", optional = true }`
  listed under `[features] ssr = [..., "dep:brdgme_crypto"]`
- Workspace members list in `rust/Cargo.toml:34`: `"lib/crypto"`

## Expected new crate structure

```
rust/lib/nats_protocol/   (or similar name, e.g. brdgme_nats)
  Cargo.toml
  src/lib.rs              (types + constants + golden-fixture test)
```

Dependencies needed: `serde` (workspace), `uuid` (workspace, features serde),
`serde_json` (dev, for golden test). No `async-nats` - the `connect()` helper
stays in each consumer (it is infra, not wire protocol).

## Consumer rewiring

- `rust/bot/src/nats.rs`: replace type/const definitions with
  `pub use brdgme_nats::*;` (or selective re-exports). Keep `connect()` local.
- `rust/web/src/nats.rs`: replace type/const definitions with re-exports.
  Keep `connect()`, `ensure_stream_and_consumers()`, drift detection, advisory
  listener, `supervise_consumer()`, and all existing tests local.
- `rust/bot/Cargo.toml`: add `brdgme_nats = { path = "../lib/nats_protocol" }`
- `rust/web/Cargo.toml`: add `brdgme_nats = { path = "../lib/nats_protocol", optional = true }`
  and `"dep:brdgme_nats"` to the `ssr` feature list.
- `rust/Cargo.toml` workspace members: add `"lib/nats_protocol"`.

## Import sites to update

Bot (`rust/bot/src/main.rs`):
- `:17` `use nats::{BotCommandEvent, BotTurnEvent};`
- `:497` `nats::SUBJECT_COMMAND`
- `:773` `nats::CONSUMER_TURN, nats::STREAM_NAME`
- `:823` `nats::connect(...)`
- `:1039` hardcoded `5 * 60` in test

Web (`rust/web/src/game/mod.rs`):
- `:214` `crate::nats::BotTurnEvent`
- `:236` `crate::nats::SUBJECT_TURN`
- `:272` `crate::nats::CONSUMER_COMMAND, crate::nats::STREAM_NAME`
- `:358` `crate::nats::BotCommandEvent`
- `:397` `crate::nats::MAX_DELIVER`
- `:468,511` `crate::nats::MAX_TURN_ATTEMPTS`

Web (`rust/web/src/main.rs`):
- `:59` `web::nats::ensure_stream_and_consumers`
- `:89,105,109` `web::nats::supervise_consumer`, `web::nats::run_max_deliveries_advisory_listener`

## Test locations

- Shared crate: golden-fixture round-trip test in `src/lib.rs` `#[cfg(test)]`
  (serialise `BotTurnEvent`/`BotCommandEvent` to JSON, assert exact bytes match
  a pinned string; deserialise back and assert field equality).
- Bot: existing `ack_heartbeat_interval_below_ack_wait` test
  (`bot/src/main.rs:1037-1040`) must reference shared `ACK_WAIT` const.
- Web: existing tests in `web/src/nats.rs:343-548` stay put (they test
  supervisor/drift/advisory, not wire types).

## Dependency patterns (workspace Cargo.toml)

```toml
serde = { version = "1.0.229", features = ["derive"] }
uuid = "1.24.0"
serde_json = "1.0.151"
async-nats = "0.49.1"
```

## Git state

- HEAD: `77d5d0f47ed4ebb068954ce4a4e1482563d73f56`
- Branch: master (implied by linear log)
- Untracked (do NOT touch): `docs/reviews/2026-07-30-review-session/R-07-HANDOVER.md`,
  `R-08-CONTEXT-HANDOVER.md`, `R-08-REVIEW.md`, `r-10-*.md`, `review/`
- Last 3 commits: R-13 done (77d5d0f docs, afe85b2 fix)

## Decisions and gotchas

1. **Do NOT move `connect()`** into the shared crate - it pulls in
   `async-nats` which is a heavy dep and is infra, not protocol.
2. **Do NOT move `ensure_stream_and_consumers`, `supervise_consumer`, drift
   detection, or advisory listener** - these are web-only server logic.
3. **`web/src/nats.rs` is `#[cfg(feature = "ssr")]`** (declared in
   `lib.rs:39-40`). The shared crate has no feature gates.
4. **Bot's `mod nats` is private** (`main.rs:8`). After rewiring it becomes a
   thin re-export facade (like `crypto.rs`).
5. **`ACK_WAIT` promotion**: the local variable at `web/src/nats.rs:151`
   becomes a shared pub const. The bot test at `main.rs:1039` asserts
   `ACK_HEARTBEAT_INTERVAL < ACK_WAIT` using the shared const.
6. **R-15 depends on R-14** (constants move once). Do not implement R-15
   delivery semantics here.
7. **No `#[serde(default)]` on wire types today** - F-108 notes this as a
   latent risk. Adding it is in-scope if the implementer judges it safe
   (fields are all required today; adding `default` changes semantics).
   The AC does not require it; the golden fixture is the protection.
8. **Commit after completion** (owner instruction). Never push.
9. **Cargo constraints**: all commands target exactly one crate. Web: only
   `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`.
   Bot/shared: `cargo test/clippy/fmt -p <crate>` serially. No workspace
   commands, no `scripts/rust-test.sh`.

## Tracker update location

On completion, update:
- `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md:39`
  (R-14 row: status, commit, notes)
