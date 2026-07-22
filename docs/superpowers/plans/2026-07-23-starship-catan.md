# Starship Catan improvements (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Crate is `rust/game/starship-catan-1` (package `starship-catan-1`,
edition 2024) - pure Rust game logic with NO database, so its tests run
locally without Postgres/NATS (unlike the `web` crate). Sources:
`src/lib.rs`, `src/command.rs`, `src/render.rs`, `src/card.rs`. `N` is
`brdgme_markup::Node` (aliased `use brdgme_markup::Node as N;`).

## Overview + requirements

Three independent quality-of-life fixes to the Starship Catan game service.
Each is self-contained and can land in any order; none touches the schema or
the `web` crate.

Requirements (verbatim intent):
- R1 Buy-goods capacity error: when a buy fails because the goods won't fit,
  show the SPARE capacity, e.g. "not enough room for 3 food - you have room
  for 1 more food" (instead of just "not enough room for 3 food").
- R2 "next" vs "end": in TERMINAL flight cases (no moves left, OR no actions
  left, OR the sector pile is empty), REMOVE "next" from the available
  commands and only show "end". This requires DECOUPLING `can_end` from
  `can_next` (`can_end` currently just delegates to `can_next`).
- R3 "Last sectors" list: make the LEFTMOST (most recent) entry bold.

DESCOPED (do NOT implement, do NOT include in any unit):
- Build command docs (per-item descriptions for build/booster/cannon) - the
  command parser does not support per-item descriptions well.
- Static cost text for booster/cannon - descoped together with build docs.

---

## 1. Current behaviour

### Transaction errors and goods capacity (`lib.rs`)

- `Transaction::cannot_fit_error` (`lib.rs:164-166`):
  `GameError::invalid_input(format!("not enough room for {}", self.gain_plain()))`.
- `gain_plain` (`lib.rs:142-149`) joins positive entries via
  `Transaction::amount_plain` (`lib.rs:89-95`), which formats a non-Astro
  resource as `"{amount} {name}"` (e.g. "3 food") and Astro as `"${amount}"`.
- `PlayerBoard::goods_limit` (`lib.rs:317-319`) = `2 + self.module(Module::Logistics)`.
- `PlayerBoard::res(r)` (`lib.rs:261-263`) reads a resource count (0 default).
- `PlayerBoard::can_fit` (`lib.rs:325-329`) compares the transaction against
  `fit_transaction` (`lib.rs:331-385`), which caps goods (Food/Fuel/Carbon/Ore/
  Trade) at `goods_limit`, Science at 4, Booster/Cannon at 6, and total ships
  at 2.
- `Resource::name` (`card.rs:41-56`) gives the lowercase display name
  ("food", "fuel", "carbon", "ore", "trade", ...).

`cannot_fit_error` has THREE call sites:
1. Flight-phase trade-card BUY - inside `can_trade` (`lib.rs:937-943`):
   builds `t` with `{resource: amount}`, and if `!can_fit(&t)` returns
   `(false, 0, t.cannot_fit_error().to_string())` (`lib.rs:941`). **R1 target.**
2. TradeAndBuild trading-post BUY - inside `can_trade` (`lib.rs:981-992`):
   same pattern, `t.cannot_fit_error().to_string()` at `lib.rs:985`. **R1 target.**
3. `build` (`lib.rs:1381`) - `return Err(t.cannot_fit_error())` when a built
   ship/booster/cannon won't fit. **OUT OF SCOPE for R1 - leave unchanged.**

`can_trade` signature is `pub fn can_trade(&self, player: usize, resource:
Resource, amount: i32) -> (bool, i32, String)` (`lib.rs:856`) - note `&self`,
so the buy sites can immutably read `self.player_boards[player]` to compute
spare capacity with no borrow conflict. `trade` (`lib.rs:1024-1034`) calls
`can_trade` and wraps the returned reason string in `GameError::invalid_input`,
so the message reaches the player verbatim.

The bought resource is always a GOOD: `tradable_resources` (`lib.rs:597-613`)
returns the current trade card's `resources` (Flight) or the trading-post
resources (TradeAndBuild), both of which are goods. So `goods_limit -
res(resource)` is the correct spare capacity for R1.

### "next" / "end" (`lib.rs`, `command.rs`)

- `next_sector_card` (`lib.rs:750-774`) is the handler behind "next". It
  computes `pile_empty` (`lib.rs:754-758`: the current sector's pile is empty
  or absent) and, if `pile_empty || self.remaining_moves() <= 0 ||
  self.remaining_actions() <= 0` (`lib.rs:759`), it calls `end_flight()`
  (`lib.rs:760`) instead of drawing a card. So today, typing "next" in a
  terminal case silently ends the flight.
- `remaining_moves` (`lib.rs:580-582`) = `flight_distance() - flight_cards.len()`;
  `flight_distance` (`lib.rs:576-578`) = `yellow_dice + res(Resource::Booster)`.
- `remaining_actions` (`lib.rs:584-587`) = `actions() - used`; `actions`
  (`lib.rs:273-275`) = `2 + module(Module::Command)`; `used` = count of `true`
  values in `flight_actions: BTreeMap<usize, bool>` (`lib.rs:494`).
- `can_next` (`lib.rs:1175-1187`): true when it is the player's turn, phase is
  `Flight`, `flight_cards` is non-empty, no `gain_resources` pending, AND
  (`card_finished` OR the top flight card does NOT `requires_action()`). This
  is the "free to act in flight" precondition. It does NOT check the terminal
  conditions - those live only in the `next_sector_card` handler.
- `can_end` (`lib.rs:1189-1191`): `self.can_next(player)` - a pure delegate.
  This is what R2 must decouple.
- Handlers: `next` (`lib.rs:1471-1478`) guards on `can_next` then calls
  `next_sector_card`; `end` (`lib.rs:1480-1487`) guards on `can_end` then calls
  `end_flight`.
- Command surface (`command.rs`, in `command_parser`): "next" is pushed only
  when `can_next` (`command.rs:302-307`); "end" is pushed only when `can_end`
  (`command.rs:309-314`). They are gated INDEPENDENTLY, so once `can_next` and
  `can_end` diverge, the command list updates automatically - **`command.rs`
  needs no change for R2.**
- Verified consumers of `can_next`/`can_end` in this crate are ONLY
  `command.rs:302,309` and `lib.rs:1472,1481` (plus the defs). No bot/AI code
  calls them. (`acquire-1` and `roll-through-the-ages-2` have their own
  unrelated `can_next`/`can_end` - untouched.)

### "Last sectors" render (`render.rs`)

- `PlayerBoard.last_sectors: Vec<i32>` (`lib.rs:234`). `end_flight` inserts the
  just-left sector at index 0 (`lib.rs:787-789`), so **index 0 = most recent**
  and the list is most-recent-first.
- The "Last sectors" row is rendered only in `Phase::ChooseSector`
  (`render.rs:126-138`): it joins ALL entries with a single space into one
  PLAIN string (`render.rs:128-133`) and puts it in the row's right cell
  (`render.rs:134-137`). R3 wants the leftmost (index 0, most recent) entry
  bold and the rest plain.
- `N::Bold(Vec<N>)` is available throughout `render.rs` (e.g. `render.rs:122`).
- Render is exposed via the `Renderer` trait: `impl Renderer for PubState`
  (`render.rs:58-62`, viewer defaults to player 0) and `impl Renderer for
  PlayerState` (`render.rs:64-68`). `brdgme_markup::to_string(&[Node])`
  (`rust/lib/markup/src/lib.rs:39-44`) serializes `Bold` as `{{b}}...{{/b}}`,
  which gives a clean assertion target for tests.

### Test conventions (`lib.rs:2055+`)

- Unit tests live in `#[cfg(test)] mod tests` in `lib.rs`. They start a game
  with `Game::start(2, 1)` (a test-only wrapper, `lib.rs:1798`) and drive it
  with `g.command(player, "command string", &players)` (`lib.rs:1834`), which
  parses + dispatches and returns `Result<CommandResponse, GameError>`.
- All `Game` fields are `pub` (`lib.rs:487-510`), and existing tests mutate
  state directly (e.g. `g.player_boards[0].colonies = vec![...];` at
  `lib.rs:2166`). New tests can likewise set `g.phase`, `g.flight_cards`,
  `g.current_player`, `g.current_sector`, `g.sector_cards`, `g.yellow_dice`,
  `g.flight_actions`, `g.card_finished`, and `g.player_boards[p].*` directly.
- A `colony_card()` test helper exists (`lib.rs:2063-2070`).
- `SectorCard::Trade` fields (`card.rs:259-266`): `{ name: String, resources:
  Vec<Resource>, price: i32, maximum: i32, direction: TradeDir, trading_post:
  bool }`.
- `SectorCard::requires_action` (`card.rs:423-425`) is true ONLY for `Pirate`.
  Any Colony/Trade/Empty/Median card satisfies the "free to act" precondition
  in `can_end`/`can_next` when `card_finished` is false.

---

## 2. Implementation units

The three units are independent (no shared code, no ordering constraint). Each
ends with fmt + clippy green and its own commit. Suggested commit order is just
S1, S2, S3.

### S1. Buy-goods capacity error shows spare capacity (R1)

- Goal: when a buy fails because the goods won't fit, tell the player how much
  room is actually free: "not enough room for 3 food - you have room for 1 more
  food".
- Files: `rust/game/starship-catan-1/src/lib.rs` only.
- Change:
  - Add a helper on `PlayerBoard` (next to `goods_limit`, ~`lib.rs:317`):
    ```rust
    pub fn cannot_fit_buy_error(&self, resource: Resource, amount: i32) -> String {
        let spare = (self.goods_limit() - self.res(resource)).max(0);
        format!(
            "not enough room for {} {} - you have room for {} more {}",
            amount,
            resource.name(),
            spare,
            resource.name()
        )
    }
    ```
    (`amount` is the positive buy quantity; `resource.name()` is the lowercase
    name. The bought resource is always a good, so `goods_limit` is the right
    cap. `.max(0)` guards against a negative spare if state is ever odd.)
  - Flight-phase buy site (`lib.rs:937-943`): replace
    `t.cannot_fit_error().to_string()` (`lib.rs:941`) with
    `self.player_boards[player].cannot_fit_buy_error(resource, amount)`.
  - TradeAndBuild buy site (`lib.rs:981-992`): replace
    `t.cannot_fit_error().to_string()` (`lib.rs:985`) with the same
    `self.player_boards[player].cannot_fit_buy_error(resource, amount)`.
  - DO NOT change `Transaction::cannot_fit_error` (`lib.rs:164-166`) and DO NOT
    touch the `build` call site (`lib.rs:1381`) - build's "not enough room"
    message is out of scope and its capacity involves the shared ship cap, so
    it must NOT be routed through the goods helper. The fix is purely additive.
- Acceptance:
  - Buying more of a good than fits (Flight trade card OR TradeAndBuild trading
    post) returns "not enough room for {amount} {resource} - you have room for
    {spare} more {resource}", where spare = `goods_limit - res(resource)`.
  - A buy that fits is unchanged; `build` errors are byte-for-byte unchanged.
  - `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` green.
- Tests (in `lib.rs` `mod tests`):
  - Direct helper test: construct a `PlayerBoard` (e.g. `PlayerBoard::new(0)`),
    set its goods near the limit, assert `cannot_fit_buy_error(Resource::Food, 3)`
    equals the expected string (default `goods_limit` is 2; with 1 food held,
    spare is 1 -> "not enough room for 3 food - you have room for 1 more food").
  - Integration through `can_trade`/`command` for the TradeAndBuild path:
    `Game::start(2, 1)`, set `g.phase = Phase::TradeAndBuild`,
    `g.current_player = 0`, give player 0 a food trading post
    (`g.player_boards[0].trading_posts = vec![SectorCard::Trade { name: ...,
    resources: vec![Resource::Food], price: 1, maximum: 0, direction:
    TradeDir::Both, trading_post: true }]`), fill food to `goods_limit - 1`,
    give enough Astro, then `g.command(0, "buy 3 food", &players)` and assert
    the error string contains "you have room for 1 more food".
  - The Flight path shares the helper; one direct helper test plus the
    TradeAndBuild integration test cover both call sites. (Optionally add a
    Flight integration test by setting `g.phase = Phase::Flight` and
    `g.flight_cards = vec![SectorCard::Trade { .. }]`.)
- Depends on: nothing.

### S2. Decouple can_next/can_end; hide "next" in terminal flight cases (R2)

- Goal: in terminal flight cases (sector pile empty, OR `remaining_moves() <= 0`,
  OR `remaining_actions() <= 0`), remove "next" from the available commands and
  show only "end". Decouple `can_end` from `can_next`.
- Files: `rust/game/starship-catan-1/src/lib.rs` only. (`command.rs` already
  gates "next" and "end" independently and needs NO change.)
- Change:
  - Add a private terminal helper mirroring `next_sector_card`'s exact
    condition (`lib.rs:754-759`):
    ```rust
    fn flight_terminal(&self) -> bool {
        let pile_empty = self
            .sector_cards
            .get(&self.current_sector)
            .map(|v| v.is_empty())
            .unwrap_or(true);
        pile_empty || self.remaining_moves() <= 0 || self.remaining_actions() <= 0
    }
    ```
  - Move the CURRENT `can_next` body (`lib.rs:1175-1187`) into `can_end`
    (replacing the delegate at `lib.rs:1189-1191`). This is the "free to act in
    flight" precondition and is when a player may end the flight - valid in BOTH
    terminal and non-terminal cases:
    ```rust
    pub fn can_end(&self, player: usize) -> bool {
        if self.current_player != player
            || self.phase != Phase::Flight
            || self.flight_cards.is_empty()
            || self.gain_resources.is_some()
        {
            return false;
        }
        if self.card_finished {
            return true;
        }
        !self.flight_cards.last().unwrap().requires_action()
    }
    ```
  - Redefine `can_next` as "can end AND not terminal":
    ```rust
    pub fn can_next(&self, player: usize) -> bool {
        self.can_end(player) && !self.flight_terminal()
    }
    ```
  - Leave the `next_sector_card` handler's terminal auto-end branch
    (`lib.rs:759-761`) in place as defensive code. With the new `can_next`, the
    `next` handler (`lib.rs:1471-1478`) rejects before reaching it in terminal
    cases, so that branch becomes unreachable via commands - but do NOT remove
    it as part of this change (keep handler behaviour stable if called
    programmatically).
- Acceptance:
  - Non-terminal flight (pile has cards, moves > 0, actions > 0): both "next"
    and "end" are available (unchanged from today).
  - Terminal flight (pile empty, OR moves <= 0, OR actions <= 0): "next" is NOT
    offered and `g.command(p, "next", ..)` errors with "you can't advance to the
    next card"; "end" IS offered and `g.command(p, "end", ..)` succeeds and ends
    the flight.
  - `can_end` no longer references `can_next`.
  - `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` green.
- Tests (in `lib.rs` `mod tests`): set up a Flight state -
  `g.phase = Phase::Flight`, `g.current_player = 0`,
  `g.flight_cards = vec![colony_card()]` (a non-Pirate card so
  `requires_action()` is false and `can_end` can be true), `g.current_sector`
  set to a sector with a non-empty `g.sector_cards` pile, and `g.yellow_dice` /
  boosters high enough that `remaining_moves() > 0` (e.g. `yellow_dice = 3`),
  `g.flight_actions` empty so `remaining_actions() > 0`. Then:
  - Non-terminal: assert `g.can_next(0)` and `g.can_end(0)` are both true.
  - Terminal via empty pile: `g.sector_cards.insert(g.current_sector, vec![])`
    -> assert `can_next(0) == false` AND `can_end(0) == true`.
  - Terminal via no moves: restore the pile, set `g.yellow_dice = 0` and
    `g.player_boards[0].resources` Booster to 0 (so `flight_distance() == 0`
    and, with one flight card, `remaining_moves() < 0`) -> `can_next` false,
    `can_end` true.
  - Terminal via no actions: restore moves, set
    `g.flight_actions = BTreeMap::from([(0, true), (1, true)])` (no Command
    module => `actions() == 2`, so `remaining_actions() == 0`) -> `can_next`
    false, `can_end` true.
  - Command-level: in a terminal state, `g.command(0, "next", &players)` is an
    `Err` and `g.command(0, "end", &players)` is `Ok`.
- Depends on: nothing.

### S3. Bold the most-recent (leftmost) "Last sectors" entry (R3)

- Goal: in the "Last sectors" row (ChooseSector phase), render the leftmost
  entry (index 0, most recent) bold and the remaining entries plain.
- Files: `rust/game/starship-catan-1/src/render.rs` only.
- Change: replace the plain join in the `Phase::ChooseSector` branch
  (`render.rs:126-138`) with a node list that bolds index 0 and keeps the
  single-space separator:
  ```rust
  Phase::ChooseSector => {
      if !boards[viewer].last_sectors.is_empty() {
          let sectors = &boards[viewer].last_sectors;
          let mut nodes = vec![N::Bold(vec![N::text(sectors[0].to_string())])];
          for s in &sectors[1..] {
              nodes.push(N::text(format!(" {}", s)));
          }
          turn_rows.push(vec![
              (A::Left, vec![N::Bold(vec![N::text("Last sectors")])]),
              (A::Left, nodes),
          ]);
      }
  }
  ```
  Index 0 is the most recent (`end_flight` inserts at 0, `lib.rs:787-789`).
  The empty-list guard is preserved (row hidden when there are no last sectors).
- Acceptance:
  - With `last_sectors = [3, 1, 2]`, the rendered "Last sectors" value is
    `3` (bold) followed by ` 1 2` (plain) - i.e. only the leftmost entry is bold.
  - Single-space separation between entries is preserved; the "Last sectors"
    label stays bold; an empty `last_sectors` still hides the row.
  - `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` green.
- Tests (in `lib.rs` `mod tests`, or a small `render.rs` test module): build a
  game, set `g.phase = Phase::ChooseSector` and
  `g.player_boards[0].last_sectors = vec![3, 1, 2]`, render via the `Renderer`
  trait (`use brdgme_game::Renderer;` then `g.pub_state().render()`, which
  renders for viewer 0), serialize with `brdgme_markup::to_string(&nodes)`, and
  assert the output contains `{{b}}3{{/b}} 1 2` (leftmost bold, rest plain).
  Also assert an empty `last_sectors` produces no "Last sectors" row.
- Depends on: nothing.

---

## 3. Decisions (all resolved - no open user questions)

1. **Build command docs** - DESCOPED. The command parser does not support
   per-item descriptions well. Not part of this plan; do not implement.
2. **Static booster/cannon cost text** - DESCOPED together with build docs. N/A.
3. **Buy-goods capacity error wording (R1)** - RESOLVED:
   "not enough room for 3 food - you have room for 1 more food". Implemented via
   a new `PlayerBoard::cannot_fit_buy_error` helper used at both buy call sites;
   the generic `Transaction::cannot_fit_error` and the `build` call site are
   left unchanged.
4. **"next" vs "end" (R2)** - RESOLVED: in terminal cases remove "next" and show
   only "end"; decouple `can_end` from `can_next` (`can_end` keeps the "free to
   act in flight" precondition; `can_next` = `can_end && !flight_terminal`).
5. **"Last sectors" bold (R3)** - RESOLVED: bold the leftmost (most recent,
   index 0) entry only.

(Implementation detail the dev Lead owns, not a user decision: the buy message
is centralized in the `PlayerBoard::cannot_fit_buy_error` helper rather than
inlined at both call sites, to keep the wording in one place. If the Lead
prefers inline formatting, both sites must stay byte-identical.)

---

## 4. Known issues / gotchas (carry forward to every Lead)

- **No database.** This is a pure game-logic crate; `cargo test -p
  starship-catan-1` runs locally with no Postgres/NATS. The DB-test caveats in
  AGENTS.md (backlog #40) do NOT apply here.
- **Target the single package** (AGENTS.md): never run workspace-wide builds/
  tests. Canonical gates before commit:
  `cargo fmt --all -- --check`;
  `cargo clippy -p starship-catan-1 --all-targets -- -D warnings`;
  `cargo test -p starship-catan-1`.
  Per AGENTS.md, run `scripts/rust-test.sh` before committing any Rust change
  (it runs fmt + clippy + tests); for this crate the targeted commands above are
  the substantive gate.
- **`cannot_fit_error` has a THIRD caller** (`build`, `lib.rs:1381`). The R1 fix
  MUST be additive (new helper) so the build message and the generic
  `Transaction::cannot_fit_error` are untouched. Do not change
  `cannot_fit_error`'s signature.
- **Bought resource is always a good** (`tradable_resources`, `lib.rs:597-613`),
  so `goods_limit - res(resource)` is correct for R1. If a future trade card
  ever sold Science/Booster/Cannon/ships, the spare calc would need the
  per-resource caps from `fit_transaction` (`lib.rs:331-385`) - not the case
  today; do not pre-engineer for it.
- **`can_end` consumers are only `command.rs:309` and `lib.rs:1481`** (verified).
  After decoupling, confirm nothing else assumed `can_end == can_next`. The
  `acquire-1` and `roll-through-the-ages-2` crates have their own unrelated
  `can_next`/`can_end` - do not touch them.
- **Keep `next_sector_card`'s terminal auto-end branch** (`lib.rs:759-761`) as
  defensive code; do not "clean it up" by deleting it as part of S2.
- **`requires_action()` is true only for `Pirate`** (`card.rs:423-425`). When
  testing `can_end`/`can_next`, use a non-Pirate flight card (Colony/Trade/
  Empty/Median) so the "free to act" precondition can hold; a Pirate card makes
  both false regardless of terminal state.
- **`Game` fields are `pub`** - tests mutate state directly (existing pattern,
  `lib.rs:2166`). Use this to set up Flight/TradeAndBuild/ChooseSector states
  deterministically rather than playing out a full game.
- **Markup:** `N = brdgme_markup::Node`; `brdgme_markup::to_string` serializes
  `Bold` as `{{b}}...{{/b}}` (`rust/lib/markup/src/lib.rs:39-44`) - a clean
  assertion target for the S3 render test.
- **Migrations:** none. This plan makes no schema change; the AGENTS.md
  migration-immutability rule is not engaged.
- **Org is `brdgme`** (not `beefsack`) for any image/URL references.
