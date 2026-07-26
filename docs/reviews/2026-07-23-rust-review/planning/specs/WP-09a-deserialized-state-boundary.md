# WP-09a: requester-boundary trust hardening

**Findings:** e F18 (major), e F36 (major). **Routed in by LEAD RULING:** the
workspace-wide `Gamer::player_state` totality gap (from WP-21), acquire-1's two
`panic!("must be Phase::SellOrTrade")` (from WP-19), sushizock-2's unbounded
`Player {}` `target` index (from WP-21). **Decision:** D-36 answered option A -
bounds-check the player index at the requester boundary **plus** a per-game
`validate` hook run after deserialization.

**Crate list** (extends `work-packages.md`'s WP-09 paths): `rust/lib/cmd`,
`rust/lib/game`, `rust/game/acquire-1`, `rust/game/sushizock-2`.

**Landing order:** WP-09a lands **first**, before the Phase 3 per-crate work and
before WP-09b. WP-21 Task 10 later refactors `steal_blue`/`steal_red` into a
shared helper - it must carry the guard this package adds, not drop it.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This tree is under
> concurrent edit; line numbers are deliberately not cited.

## 1. Problem

- **e F18** (`rust/game/lost-cities-2/src/lib.rs`, `Gamer::player_state`) and
  **e F36** (`rust/game/lost-cities-1/src/lib.rs`, same fn) index
  `self.hands[player]` with the player index taken verbatim from the request.
- **Routed - `Gamer::player_state` totality gap** (`rust/lib/game/src/game.rs`):
  the trait signature is `fn player_state(&self, player: usize) -> Self::PlayerState`
  with no way to reject an out-of-range player, so ~30 crates must each make
  their renderer total or panic.
- **Routed - acquire-1** (`rust/game/acquire-1/src/lib.rs`,
  `next_player_sell_trade` and `end_sell_trade_phase`): each matches
  `self.phase` and `panic!("must be Phase::SellOrTrade")` on any other arm.
- **Routed - sushizock-2** (`rust/game/sushizock-2/src/lib.rs`, `steal_blue`
  and `steal_red`): `target` comes from the `Player {}` parser, which bounds it
  by `names.len()`, not by `self.players`. Both then index
  `self.player_blue_tiles[target]` / `self.player_red_tiles[target]`.

## 2. Why it's wrong

- **e F18 and e F36 are correct as written.** Verified live:
  `GameRequester::request` (`rust/lib/cmd/src/requester/gamer.rs`) deserializes
  `Request::PlayerRender { player, game }` and `handle_player_render` forwards
  `player` straight into `game.player_state(player)`. Nothing between the wire
  and the index checks the bound. Same for `Request::Play` into `handle_play`.
  Do not revert either finding.
- The totality gap is real but the *signature* is not the defect: those two
  handlers are the only unchecked callers. `gamer.rs::renders` iterates
  `0..game.player_count()` and `rust/lib/game/src/bot.rs` picks from
  `game.whose_turn()` - both already bounded.
- acquire-1's two panics are unreachable through normal play (every caller has
  just matched or just assigned `Phase::SellOrTrade`), but they are `panic!` in
  a function reachable from `Gamer::command` and both already return
  `Result<_, GameError>`, so the conversion is free.
- sushizock-2 checks `player == target` and emptiness but never
  `target < self.players`.

## 3. Required end state

### 3a. The design ruling - boundary check only, signature unchanged

**Do NOT change `Gamer::player_state`'s signature.** Making it return
`Result`/`Option` would edit ~30 crates for no gain: every caller other than
the two request handlers is already bounded by `player_count()` or
`whose_turn()`. The check lives in `gamer.rs`, in one place.

### 3b. `rust/lib/game/src/game.rs` - the `validate` hook

Add one defaulted method to `trait Gamer`, next to `assert_not_finished`:

```rust
/// Called on state that has just been deserialized from an untrusted or
/// stored blob, before any other method. Default is a no-op; games with
/// cross-field invariants should override it.
fn validate(&self) -> Result<(), GameError> {
    Ok(())
}
```

No game crate implements it in this package. Do not add `validate` impls to
any crate here.

### 3c. `rust/lib/cmd/src/requester/gamer.rs` - the boundary

- Add a private helper, e.g.
  `fn check_player<G: Gamer>(player: usize, game: &G) -> Option<Response>`,
  returning `Some(Response::UserError { .. })` when `player >= game.player_count()`
  and `None` otherwise. Message names the bound, e.g.
  `"invalid player {player}, game has {n} players"`.
- In `Requester::request`, for **both** `Request::Play` and
  `Request::PlayerRender`: after `serde_json::from_str`, call `game.validate()`
  and return `Response::SystemError { message: e.to_string() }` on `Err`; then
  run `check_player` and return its `Response` if `Some`; only then call
  `handle_play` / `handle_player_render`.
- `Request::Status` and `Request::PubRender` also deserialize a game - call
  `game.validate()` there too (they carry no player index, so no bounds check).
- `handle_play` and `handle_player_render` keep their current signatures and
  bodies. `renders` is unchanged.

### 3d. `rust/game/acquire-1/src/lib.rs`

In `next_player_sell_trade` and `end_sell_trade_phase`, replace each
`_ => panic!("must be Phase::SellOrTrade")` arm with
`_ => return Err(GameError::internal("expected Phase::SellOrTrade"))`
(`GameError::internal` exists in `rust/lib/game/src/errors.rs`). Both fns
already return `Result<(Vec<Log>, bool), GameError>`; no caller changes.

### 3e. `rust/game/sushizock-2/src/lib.rs`

In **both** `steal_blue` and `steal_red`, immediately after the existing
`if player == target` self-steal check, add:

```rust
if target >= self.players {
    return Err(GameError::invalid_input("that is not a player in this game"));
}
```

Leave the rest of both functions alone.

## 4. Non-goals

- **WP-09b** - the per-crate defensive `.get()` sweep of WP-09's remaining 17
  minor/nit findings across ~13 crates. Not this package; do not touch them.
- **WP-28 Task 3** deliberately keeps `self.hands[player]` panicking in both
  lost-cities crates so this package's red test stays reproducible. **Do not
  "fix" it and do not widen WP-28.** After 3c lands the panic is unreachable
  from the request path; the indexing form stays.
- **WP-06** (lib/cmd tools and http) is finalized and does **not** carry this
  bounds check. Do not retro-edit it.
- **WP-10** is the *outbound* redaction direction (`pub_state` hidden info);
  WP-09a is *inbound* trust only. No `pub_state` changes.
- **WP-21 Task 10** must not be pre-empted: add sushizock-2's guard only, do
  not refactor its take/steal pairs.
- All rules parity (WP-11/12/16/20/26/30 are parked). No gameplay change, no
  `RULES.md` or `DATA_DOCS.md` edits.
- No changes to `Gamer::player_state`, `Gamer::command`, or any game crate's
  `player_state` impl.

## 5. Regression test cases

- `rust/lib/cmd/src/requester/gamer.rs` - add a `#[cfg(test)] mod tests` (the
  crate's only existing test module is in `rust/lib/cmd/src/api.rs`; copy its
  style). Use a minimal in-module `Gamer` stub whose `player_state` panics out
  of range:
  - `Request::PlayerRender` with `player == player_count()` returns
    `Response::UserError`, not a panic. **This is the red test for e F18/e F36**
    and must be shown failing before 3c lands. Same for `Request::Play`.
  - An in-range `player` still yields `Response::PlayerRender` /
    `Response::Play` unchanged.
  - A stub whose `validate` returns `Err` yields `Response::SystemError` for
    `Play`, `PlayerRender`, `Status` and `PubRender`.
- `rust/game/sushizock-2/src/lib.rs` `mod test` (existing, near the file end -
  holds `test_take_worst_red_picks_minimum`): `steal_blue` and `steal_red` with
  `target == self.players` return `Err(GameError::InvalidInput { .. })`, not a
  panic; an in-range steal still succeeds.
- `rust/game/acquire-1/src/lib.rs` `mod tests` (existing): a `Game` whose
  `phase` is not `Phase::SellOrTrade` passed to `end_sell_trade_phase` returns
  `Err(GameError::Internal { .. })`. Existing sell/trade/keep/merge tests stay
  green - the reachable paths are unchanged.
- `rust/lib/game/src/game.rs` `mod tests` (existing, holds
  `gen_placings_works`): a `Gamer` impl that does not override `validate`
  returns `Ok(())`.

## 6. Riders

None - see WP-09b for the remaining 17 minor/nit findings.
