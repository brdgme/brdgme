# WP-13: starship-catan-1 fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Fix the five majors in `rust/game/starship-catan-1` — the cannon-cost surcharge keyed off boosters (a F11), the `can_lose_module` `||` that lets a player voluntarily sacrifice a module to skip any pirate (a F12), the missing astro-affordability check on TradeAndBuild buys (a F13), the debug-build i32 overflow reachable from unbounded `buy`/`sell` amounts (a F14), and the Sensor peek that is never rendered to the peeking player (a F15). Also land the verified minors/nits: current-turn row shows the viewer instead of the current player (a F16), dead code removal (a F17), the inverted direction-mismatch error message (a F18), and the unbounded `last_sectors` history (a F19). a F20 (`flight_actions` BTreeMap-as-set) is SKIPPED — the change is serde-incompatible with persisted game states (reasoning below).

**Architecture — how starship-catan-1 works (read this before editing):**

- One crate, `rust/game/starship-catan-1` (package name `starship-catan-1`, confirmed from `Cargo.toml`): `src/lib.rs` (game state machine, `Transaction`/fit machinery, `Gamer` impl, inline tests at lib.rs:2153), `src/card.rs` (resources, modules, sector/adventure card data), `src/command.rs` (command parsers, gated by the `can_*` guards), `src/render.rs` (`PubState`/`PlayerState` + markup rendering), `tests/contract.rs` (standard contract harness — untouched).
- 2-player only. Turn loop: `Phase::ChooseModule` (once, at start) → per turn `Produce` (yellow die 1-3, colony/module production, multi-choice gains via `gain_resources`/`gain_queue`) → `ChooseSector` (pick sector 1-4; a Sensor module pops 2-3 cards into `Game::peeking` for reordering via `put <#> top|bottom`) → `Flight` (draw sector cards one at a time; trade / found / fight / pay / lose / complete; ends via `end_flight`) → `TradeAndBuild` (up to 2 trading-post trades, `take` via Trade module, `build`, `upgrade`, `done` → opponent's turn).
- Economy: `PlayerBoard::resources: BTreeMap<Resource, i32>`. All resource mutation SHOULD go through `Transaction` + `fit_transaction`/`transact` (which clamp gains to capacity), but `Game::trade` (lib.rs:1064-1066) and `Game::pay` debit `res_mut` directly — which is why a missing affordability guard (a F13) lets astro go negative. `can_afford` = the losses fit (i.e. would not clamp), `can_fit` = the whole transaction fits.
- Parser gating: `command_parser` (command.rs:144-322) only offers a verb when its `can_*` guard passes; every action re-checks its guard on execution. A wrong guard therefore both offers the verb in the UI AND accepts it (a F12).
- Serialization: the whole `Game` is serde round-tripped between requests (DB stores game_state JSON strings). `PlayerBoard`, `SectorCard`, `flight_actions: BTreeMap<usize, bool>` etc. are persisted shapes. **No fix in this package may break deserialization of existing states.** The only serialized-shape edit in this package is the removal of the write-only `start_card: bool` field (Task 9), which is compatible because serde ignores unknown JSON fields on deserialize by default (no `deny_unknown_fields` anywhere in the crate) and nothing ever reads it.
- Rendering: `PubState::render` calls `render(self, None, None)`; `PlayerState::render` calls `render(&self.public, Some(self.player), Some(&self.peeking))` (render.rs:58-68). `brdgme_markup::to_string` serializes `N::Player(p)` as `{{player p}}` and `N::Bold` as `{{b}}...{{/b}}` — existing render tests (e.g. `last_sectors_leftmost_bold`, lib.rs:2471) assert on those markers.

**Tech Stack:** Rust 1.97.0 (edition 2024, let-chains in use) workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `starship-catan-1`. Tests: inline `#[cfg(test)] mod tests` in `src/lib.rs` (existing, line 2153) plus the untouched `tests/contract.rs`.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p starship-catan-1`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- All existing tests MUST keep passing unmodified, with ONE sanctioned exception: Task 9 deletes the `start_card: false` line from the `colony_card()` test helper (lib.rs:2166) because the field itself is deleted. Any other existing-test failure means a fix is wrong — stop and re-check.
- No serialized-shape changes except the Task 9 field removal justified above. `flight_actions`, `last_sectors`, `peeking`, `Transaction`, `PlayerBoard` all keep their exact serde shapes.
- Line numbers cited below are live-file numbers as of the drift check. Earlier tasks shift later lib.rs numbers by a few lines — always locate by the quoted symbol/format string, not by count.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).

**Non-Goals:**

- a F20 (`flight_actions: BTreeMap<usize, bool>` → `BTreeSet<usize>`) — SKIPPED, serde-incompatible (see re-derivation note 4 and the disposition table). Only a documenting comment lands (Task 9).
- `player_state()` serves `Game::peeking` to BOTH players' `PlayerState` JSON (lib.rs:1854-1860) — the opponent's API/bot payload can see the peeked cards. That is a hidden-info exposure of the WP-10 redaction class (decision D-33), NOT part of a F15, and is not fixed here; Task 5 only guards the human render so the fix does not widen the leak. Recorded as a cross-package coordination point.
- Deserialized-state trust hardening (out-of-range `current_player` etc. from crafted saved states) — WP-09, blocked on D-36. Do not add bounds checks beyond what the tasks specify.
- lib-game parser behavior (`Int` parsing, suggest caps, OneOf ranking) — WP-03/WP-04. Task 3 only changes which `Int` constructor this crate calls.
- The five copy-pasted `if self.is_finished()` placings epilogues in `command()` (lib.rs:1889-1898 etc.) — not a batch-a finding; leave them.
- The "Last sectors" row rendering `boards[viewer]` (render.rs:129) — viewer's own history is plausibly intentional and no finding covers it. a F16 is ONLY the "Current turn:" row (render.rs:125).

**Snapshot drift:** None. `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/starship-catan-1 /home/beefsack/Development/brdgme/rust/game/starship-catan-1` is empty (verified 2026-07-25 against snapshot commit f8763a5). All line numbers below are live-file line numbers and match the findings' citations.

**Re-derivation notes (differences from / sharpenings of the findings — read before the tasks):**

1. **a F14 fix choice (Task 3):** the finding offered checked arithmetic OR a parser cap. Re-derivation picks the parser cap `Int::bounded(1, 99)`: (a) it is one choke point per verb instead of guarding four arithmetic sites (`can_trade` lib.rs:920 `amount * sign + trade_amount`, lib.rs:938 `amount * price`, `fit_transaction` lib.rs:362 and lib.rs:373 `cur + v`, `trade` lib.rs:1064); (b) the largest LEGAL amount is 4 (goods capacity is `2 + Logistics level` ≤ 4, science cap 4, sell bounded by holdings ≤ 4), so 99 is generous margin and amounts 5-99 still reach the existing, informative downstream errors ("you only have $N", "not enough room for ..."); (c) with amount ≤ 99 every product/sum is tiny (max price in the card data is 5 → `amount * price` ≤ 495; `cur + v` ≤ 4 + 99); (d) suggest is unaffected — `suggest.rs:85-96` already caps Int enumeration at `min + 4`, identical for `positive()` and `bounded(1, 99)`. Verification's sharpened reachability stands in live code: several trade cards have `maximum: 0` (all `trade()` constructor cards, card.rs:472-481 and the data at card.rs:566-586), which skips the lib.rs:921 per-card cap, so `buy 2147483647 carbon` on e.g. "Tostoku I" (price 2) reaches `amount * price` at lib.rs:938 and panics under overflow checks.
2. **a F18 reachability (Task 7):** the misleading message at lib.rs:917 is currently NOT reachable via a full command round-trip — `can_buy`/`can_sell` call `can_trade(player, Resource::Any, ±1)`, hit the same direction check, and simply withhold the verb from the parser, discarding the message. It IS the message any direct `can_trade` caller (tests, future UI hints) receives, and the fix is a one-token swap; severity nit stands. The regression test therefore calls `can_trade` directly rather than `command()`.
3. **a F15 guard (Task 5):** rendering `peeking` unconditionally in `PlayerState::render` would EXPOSE the peeked cards to the opponent, because `player_state()` clones `self.peeking` into both players' states. Only the current player can peek (`can_put` requires `current_player == player`, lib.rs:1128-1130; `sector()` only fills `peeking` for the current player), so the render is gated on `player == Some(pub_state.current_player)`. The underlying JSON exposure stays with WP-10 (Non-Goals).
4. **a F20 serde conclusion (skip):** `flight_actions: BTreeMap<usize, bool>` (lib.rs:505) serializes in JSON as an object with string keys — e.g. `{"1":true,"2":true}`. `BTreeSet<usize>` serializes as an array — `[1,2]`. The shapes differ, so deserializing any live game state saved mid-flight would fail with a type error. A dual-shape custom `Deserialize` (accept map or array) is disproportionate for a simplicity nit, and the work-packages note explicitly allows skipping. Skipped; Task 9 adds a comment on the field naming the invariant (values are always `true`; shape kept for saved-state compatibility) so the next reader does not re-flag it.

---

### Task 1: cannon surcharge keys off cannons, not boosters (a F11, MAJOR)

**Problem (restated):** `PlayerBoard::cannon_transaction` (`rust/game/starship-catan-1/src/lib.rs:307-315`) adds the +1 science surcharge when `self.res(Resource::Booster) >= 3`:

```rust
    pub fn cannon_transaction(&self) -> Transaction {
        let mut t = BTreeMap::new();
        t.insert(Resource::Carbon, -2);
        t.insert(Resource::Cannon, 1);
        if self.res(Resource::Booster) >= 3 {
            t.insert(Resource::Science, -1);
        }
        Transaction(t)
    }
```

The parallel `booster_transaction` (lib.rs:297-305) correctly checks `Resource::Booster` — this is a copy-paste bug. RULES.md:172-173 states the surcharge in parallel per item: "build booster (2 fuel, plus 1 science once you have 3+)" / "build cannon (2 carbon, plus 1 science once you have 3+)" — 3+ of the item being built. Effect in legal play: a player with 3+ boosters and <3 cannons overpays for cannons (or is wrongly blocked when out of science); a player with 3+ cannons and <3 boosters underpays. Reached from `Game::build` (lib.rs:1401) on every `build cannon`.

**Fix (re-derived, matches the finding's recommendation):** change the condition to `self.res(Resource::Cannon) >= 3`. Nothing else — the affordability/fit checks in `build()` already consume the transaction correctly.

**Edge cases:** exactly 3 cannons → surcharge applies (`>=`, matching the booster branch); starting board (1 cannon, 2 boosters) → no surcharge either way, so the game opening is unchanged; cannon cap of 6 (`can_build_cannon`) unaffected; `fit_transaction`'s science floor still clamps correctly if science is 0 (surcharge makes the build unaffordable via `can_afford`, same as boosters today).

**Files:**
- Modify: `rust/game/starship-catan-1/src/lib.rs` (`cannon_transaction`, line 311)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/starship-catan-1/src/lib.rs`:

```rust
    #[test]
    fn cannon_surcharge_keys_off_cannons_not_boosters() {
        // a F11: the +1 science surcharge applies once you own 3+ CANNONS,
        // mirroring booster_transaction's check of its own item.
        let mut board = PlayerBoard::new(0);
        board.resources.insert(Resource::Cannon, 3);
        board.resources.insert(Resource::Booster, 0);
        assert_eq!(
            board.cannon_transaction().0.get(&Resource::Science),
            Some(&-1),
            "3 cannons must trigger the science surcharge"
        );

        let mut board = PlayerBoard::new(0);
        board.resources.insert(Resource::Cannon, 0);
        board.resources.insert(Resource::Booster, 3);
        assert_eq!(
            board.cannon_transaction().0.get(&Resource::Science),
            None,
            "boosters must not trigger the cannon surcharge"
        );
    }

    #[test]
    fn booster_surcharge_keys_off_boosters() {
        // Lock in the correct sibling so a symmetric copy-paste can't recur.
        let mut board = PlayerBoard::new(0);
        board.resources.insert(Resource::Booster, 3);
        board.resources.insert(Resource::Cannon, 0);
        assert_eq!(
            board.booster_transaction().0.get(&Resource::Science),
            Some(&-1)
        );
        let mut board = PlayerBoard::new(0);
        board.resources.insert(Resource::Booster, 2);
        board.resources.insert(Resource::Cannon, 3);
        assert_eq!(board.booster_transaction().0.get(&Resource::Science), None);
    }
```

- [ ] Run: `cargo test -p starship-catan-1 cannon_surcharge`. Expected: FAILS on both asserts of the first test (3 cannons/0 boosters yields no surcharge; 0 cannons/3 boosters yields one). `booster_surcharge_keys_off_boosters` passes already.
- [ ] Implement: in `cannon_transaction` change line 311 from `if self.res(Resource::Booster) >= 3 {` to `if self.res(Resource::Cannon) >= 3 {`.
- [ ] Run: `cargo test -p starship-catan-1` — new tests PASS, full suite PASS.
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): cannon surcharge scales off cannons (a F11, WP-13)`

---

### Task 2: can_lose_module must require a lost fight (a F12, MAJOR)

**Problem (restated):** `can_lose_module` (`rust/game/starship-catan-1/src/lib.rs:1266-1268`):

```rust
    pub fn can_lose_module(&self, player: usize) -> bool {
        self.current_player == player || self.losing_module
    }
```

For the current player the `||` makes this ALWAYS true. The parser's Pirate arm (command.rs:158-168) offers `lose` whenever this guard passes and a pirate is the top flight card, and `Game::lose` (lib.rs:1666-1690) re-checks only the same guard, then destroys a module level, clears `losing_module`, and calls `end_flight()`. So while landed on ANY pirate — before fighting, even a harmless ransom-$3 one — the current player can type `lose <module>`, sacrifice a module they choose, and end the flight, bypassing the fight/ransom decision entirely. RULES.md:152 and RULES.md:234 document `lose` only as "after losing a module-destroying fight: pick the module lost". `losing_module` is set in exactly one place — the fight-loss branch of `fight()` (lib.rs:1640-1641, only when `destroy_module` and the player has modules) — and cleared in `lose()` (lib.rs:1687), so it is the precise "lost a module-destroying fight" flag.

**Fix (re-derived, matches the finding; verification confirmed `&&` is sufficient):**

```rust
    pub fn can_lose_module(&self, player: usize) -> bool {
        self.current_player == player && self.losing_module
    }
```

No extra phase/pirate-card guards are needed: `losing_module` can only become true inside `fight()` (which requires `Phase::Flight` and a Pirate top card via `can_fight`), and nothing else advances the game while it is set — `whose_turn` stays on the current player and every other Flight verb's guard is `!self.losing_module`-gated (`can_fight` lib.rs:1246, `can_pay_ransom` lib.rs:1254) or unreachable (`can_end` requires `card_finished` or a non-action card; a pirate `requires_action()`). This matches verification's evidence note.

**Edge cases:** pirate landed, not yet fought → `lose` no longer parses (the parser omits the verb), fight/pay/nothing are the only options — the pre-fix exploit; fight lost with `destroy_module` and the player HAS modules → `losing_module = true`, `lose <module>` works exactly as before, destroys one level, ends the flight; fight lost with no modules → `losing_module` never set (guarded at lib.rs:1640), `end_flight` fires inside `fight()`, unchanged; `lose` for a module you don't own while `losing_module` → existing "you don't have that module" error (lib.rs:1672-1674), unchanged; opponent can never `lose` (needs `current_player == player`), unchanged.

**Files:**
- Modify: `rust/game/starship-catan-1/src/lib.rs` (`can_lose_module`, line 1267)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Add a pirate helper next to `colony_card()` in `mod tests` (Task 5 reuses it):

```rust
    fn pirate_card() -> SectorCard {
        SectorCard::Pirate {
            strength: 2,
            ransom: 3,
            destroy_cannon: false,
            destroy_module: true,
        }
    }
```

- [ ] Write the failing test:

```rust
    #[test]
    fn lose_rejected_without_lost_fight() {
        // a F12: `lose` must not be offered as a voluntary pirate-skip; it is
        // only legal after losing a module-destroying fight.
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.current_sector = 1;
        g.flight_cards = vec![pirate_card()];
        g.player_boards[0].modules.insert(Module::Sensor, 1);
        assert!(
            g.command(0, "lose sensor", &players).is_err(),
            "voluntary module sacrifice on an unfought pirate must be rejected"
        );
        assert_eq!(g.player_boards[0].module(Module::Sensor), 1);
        assert_eq!(g.phase, Phase::Flight, "the flight must not have ended");
    }

    #[test]
    fn lose_works_after_losing_module_fight() {
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.current_sector = 1;
        g.flight_cards = vec![pirate_card()];
        g.player_boards[0].modules.insert(Module::Sensor, 1);
        g.losing_module = true; // as set by the fight-loss branch of fight()
        g.command(0, "lose sensor", &players).unwrap();
        assert_eq!(g.player_boards[0].module(Module::Sensor), 0);
        assert!(!g.losing_module);
        assert_eq!(g.phase, Phase::TradeAndBuild, "losing the module ends the flight");
    }
```

- [ ] Run: `cargo test -p starship-catan-1 lose_`. Expected: `lose_rejected_without_lost_fight` FAILS (the command currently succeeds — module destroyed, phase becomes TradeAndBuild). `lose_works_after_losing_module_fight` passes already; it locks in that the fix does not break the legitimate path.
- [ ] Implement: in `can_lose_module` change line 1267 from `self.current_player == player || self.losing_module` to `self.current_player == player && self.losing_module`.
- [ ] Run: `cargo test -p starship-catan-1` — both tests PASS, full suite PASS.
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): lose module only after a lost fight (a F12, WP-13)`

---

### Task 3: cap buy/sell amounts at the parser (a F14, MAJOR)

**Problem (restated):** `buy_parser`/`sell_parser` (`rust/game/starship-catan-1/src/command.rs:121` and `:136`) parse the amount with `Int::positive()` — any i32 up to `i32::MAX` is admitted. `can_trade` then computes `amount * trade_dir.sign() + self.trade_amount` (lib.rs:920) and `amount * price` (lib.rs:938), `fit_transaction` computes `cur + v` (lib.rs:362 for science, lib.rs:373 for goods), and `Game::trade` computes `amount * price` (lib.rs:1064) — all plain i32 arithmetic. Reachability (verification-sharpened, re-confirmed against live data): all plain `trade()` cards have `maximum: 0` (card.rs:472-481; data card.rs:566-586), which skips the per-card amount cap at lib.rs:921, so `buy 2147483647 carbon` while landed on e.g. "Tostoku I" (price 2, card.rs:567) reaches `2147483647 * 2` at lib.rs:938 and panics in debug/CI builds. In TradeAndBuild, `buy 2147483647 <good you already hold>` reaches `cur + v` in `fit_transaction`. Per project policy a player-reachable panic is a defect even though release builds happen to reject the wrapped values downstream.

**Fix (re-derived — see re-derivation note 1 for why the parser cap beats checked arithmetic):** bound both amount parsers with `Int::bounded(1, 99)` via a named constant. 99 is far above the largest legal amount (4) yet keeps every product/sum comfortably in range; amounts 5-99 continue to produce the existing, more informative game errors; amounts >99 get the parser's "N is too high / number between 1 and 99" rejection. Suggest output is unchanged (already capped at `min + 4`).

**Edge cases:** `buy 3 food` etc. (all in-range play) → byte-identical behavior, locked by the existing `buy_over_capacity_trade_and_build` test (lib.rs:2367); `buy 100 carbon` → parse error instead of "you only have $25"-style game error (acceptable: no legal 100-buy exists); `buy 2147483647 carbon` → parse error instead of debug panic; sell symmetrical; `Opt` resource suffix untouched; `put`/`complete`/`sector` parsers untouched (their `Int`s feed comparisons only, no arithmetic — `sector` is already `bounded(1, 4)`); to_spec/doc output changes only min/max metadata, no rendered doc text.

**Files:**
- Modify: `rust/game/starship-catan-1/src/command.rs` (lines 121, 136 + new const)
- Test: `rust/game/starship-catan-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn huge_buy_amount_rejected_before_arithmetic() {
        // a F14: Int::positive() admitted i32::MAX, overflowing amount * price
        // (debug panic). maximum: 0 cards skip the per-card cap, so this card
        // shape reaches the multiply.
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.current_sector = 1;
        g.flight_cards = vec![SectorCard::Trade {
            name: "Tostoku I".to_string(),
            resources: vec![Resource::Carbon],
            price: 2,
            maximum: 0,
            direction: TradeDir::Both,
            trading_post: false,
        }];
        assert!(g.command(0, "buy 2147483647 carbon", &players).is_err());
        assert!(g.command(0, "sell 2147483647 carbon", &players).is_err());
        assert_eq!(
            g.player_boards[0].res(Resource::Astro),
            25,
            "no money may move on a rejected amount"
        );
    }
```

- [ ] Run: `cargo test -p starship-catan-1 huge_buy_amount`. Expected: FAILS by PANIC — `attempt to multiply with overflow` at lib.rs:938 (test builds carry overflow checks). The sell line alone would pass today (sell has no pre-guard multiply on this path); it is included to lock the symmetric cap.
- [ ] Implement. In `rust/game/starship-catan-1/src/command.rs`:

  1. Add above `found_parser()` (after the `PutWhere` enum):

```rust
/// Upper bound for buy/sell amounts. The largest legal amount is 4 (goods
/// capacity is 2 + Logistics level); the bound exists so player input can
/// never overflow the i32 price/fit arithmetic in lib.rs (a F14). Amounts
/// between the legal maximum and this bound still get the more informative
/// downstream game errors.
const MAX_TRADE_AMOUNT: i32 = 99;
```

  2. In `buy_parser` (line 121) and `sell_parser` (line 136), change `AfterSpace::new(Int::positive()),` to `AfterSpace::new(Int::bounded(1, MAX_TRADE_AMOUNT)),`.

- [ ] Run: `cargo test -p starship-catan-1` — new test PASSES (parse error, no panic), full suite PASSES (in particular `buy_over_capacity_trade_and_build` proves in-range amounts unchanged).
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/command.rs rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): bound buy/sell amounts to stop i32 overflow (a F14, WP-13)`

---

### Task 4: TradeAndBuild buys must check astro affordability (a F13, MAJOR)

**Problem (restated):** the `Phase::Flight` buy branch of `can_trade` checks `amount * price > astro` (lib.rs:937-947), but the `Phase::TradeAndBuild` buy branch (lib.rs:996-1011) only checks `can_fit` and that a buy price exists:

```rust
                    if trade_dir == TradeDir::Buy {
                        let mut t = Transaction::default();
                        t.0.insert(resource, amount);
                        if !self.player_boards[player].can_fit(&t) {
                            return (
                                false,
                                0,
                                self.player_boards[player].cannot_fit_buy_error(resource, amount),
                            );
                        }
                        if let Some(p) = prices.get(&resource)
                            && p.buy > 0
                        {
                            return (true, p.buy, String::new());
                        }
                        return (false, 0, "you aren't able to buy that resource".to_string());
                    }
```

`Game::trade` then debits unconditionally: `*self.player_boards[player].res_mut(Resource::Astro) -= total;` (lib.rs:1065). So `buy 2 carbon` at a trading post with $0 leaves the player at negative astro — a fully legal command sequence (own a trading post, enter TradeAndBuild, buy with insufficient funds). Astro is never floor-clamped anywhere (`res_mut` writes raw), so the corruption persists in saved state.

**Fix (re-derived, matches the finding):** add the same affordability check to the TradeAndBuild buy branch, AFTER the price lookup (the price is needed for the product; the fit check stays first, preserving today's error precedence for over-capacity buys). Post-Task-3 the product `amount * p.buy` is overflow-safe (≤ 99 × 5). Error message mirrors the Flight branch's `"you only have ${}"` exactly.

**Edge cases:** exact affordability (`amount * p.buy == astro`) → allowed, astro goes to 0 (`>` not `>=`, same as Flight); `can_buy` gating unchanged — it probes with `Resource::Any`, which skips the resource-specific block, so a broke player still sees the `buy` verb and gets the clear error on execution (identical shape to today's over-capacity path); multi-post best price: `trading_post_prices` already resolved `p.buy` before the check, so the check uses the price actually charged; sells unaffected (selling gains astro).

**Files:**
- Modify: `rust/game/starship-catan-1/src/lib.rs` (`can_trade` TradeAndBuild buy branch, lines 1006-1011)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests:

```rust
    fn test_trading_post(resource: Resource, price: i32) -> SectorCard {
        SectorCard::Trade {
            name: "Test Post".to_string(),
            resources: vec![resource],
            price,
            maximum: 0,
            direction: TradeDir::Both,
            trading_post: true,
        }
    }

    #[test]
    fn trade_and_build_buy_requires_astro() {
        // a F13: buying at a trading post with insufficient astro must fail
        // instead of driving the balance negative.
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::TradeAndBuild;
        g.current_player = 0;
        g.player_boards[0].trading_posts = vec![test_trading_post(Resource::Food, 3)];
        g.player_boards[0].resources.insert(Resource::Astro, 2);
        let result = g.command(0, "buy 1 food", &players);
        assert!(result.is_err(), "an unaffordable buy must be rejected");
        assert!(
            result.unwrap_err().to_string().contains("you only have $2"),
            "error must state the available astro"
        );
        assert_eq!(g.player_boards[0].res(Resource::Astro), 2);
        assert_eq!(g.player_boards[0].res(Resource::Food), 0);
    }

    #[test]
    fn trade_and_build_buy_allows_exact_astro() {
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::TradeAndBuild;
        g.current_player = 0;
        g.player_boards[0].trading_posts = vec![test_trading_post(Resource::Food, 3)];
        g.player_boards[0].resources.insert(Resource::Astro, 6);
        g.command(0, "buy 2 food", &players).unwrap();
        assert_eq!(g.player_boards[0].res(Resource::Astro), 0);
        assert_eq!(g.player_boards[0].res(Resource::Food), 2);
    }
```

- [ ] Run: `cargo test -p starship-catan-1 trade_and_build_buy`. Expected: `trade_and_build_buy_requires_astro` FAILS — the buy currently succeeds and astro reads -1. `trade_and_build_buy_allows_exact_astro` passes already (lock-in against over-restricting).
- [ ] Implement. In the TradeAndBuild buy branch of `can_trade`, replace the `if let Some(p) = prices.get(&resource) && p.buy > 0 { return (true, p.buy, String::new()); }` block (lib.rs:1006-1010) with:

```rust
                        if let Some(p) = prices.get(&resource)
                            && p.buy > 0
                        {
                            // a F13: the Flight branch checks affordability;
                            // this branch must too, or trade() drives astro
                            // negative via its unconditional debit.
                            if amount * p.buy > self.player_boards[player].res(Resource::Astro) {
                                return (
                                    false,
                                    0,
                                    format!(
                                        "you only have ${}",
                                        self.player_boards[player].res(Resource::Astro)
                                    ),
                                );
                            }
                            return (true, p.buy, String::new());
                        }
```

  (the trailing `return (false, 0, "you aren't able to buy that resource".to_string());` stays as-is.)

- [ ] Run: `cargo test -p starship-catan-1` — both tests PASS, full suite PASS.
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): check astro before trading-post buys (a F13, WP-13)`

---

### Task 5: render the Sensor peek to the peeking player (a F15, MAJOR)

**Problem (restated):** `PlayerState::render` passes `Some(&self.peeking)` into `render` (`rust/game/starship-catan-1/src/render.rs:66`), but the parameter is `_peeking` and unused (render.rs:108). No log shows the peeked card identities either — `sector()` logs only "is using the sensor module to peek at N cards" (lib.rs:1377-1388) and `put()` logs only "put a card on the top of the pile" (lib.rs:1350-1353). A human Sensor user must issue `put <#> top|bottom` completely blind; the module is unusable as intended.

**Fix (re-derived; guard added over the finding's recommendation — see re-derivation note 3):** render a numbered "Peeked cards" table in `render()` when (a) the viewer IS the current player and (b) `peeking` is non-empty. Numbering is 1-based `enumerate`, matching `put`'s validation `num < 1 || num > self.peeking.len()` and `self.peeking.remove(num - 1)` (lib.rs:1328-1333). Cards render via `SectorCard::full_string()` so pirate strength/destroy info is visible before choosing an order.

**Edge cases:** opponent's `PlayerState` (its `peeking` field is also populated — Non-Goals) → guard `player == Some(pub_state.current_player)` excludes it, since only the current player can be mid-peek; `PubState::render` passes `peeking = None` → excluded by the `Some` match; empty `peeking` (no Sensor, or all cards placed) → no section; after each `put` the remaining cards re-render renumbered from 1, which matches `put`'s index-into-remaining semantics exactly.

**Files:**
- Modify: `rust/game/starship-catan-1/src/render.rs` (`render` signature line 108, new block after the turn table at line 180)
- Test: `rust/game/starship-catan-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn sensor_peek_rendered_only_to_peeking_player() {
        use brdgme_game::Renderer;
        // a F15: the peeking player must see the peeked cards, numbered to
        // match `put <#>`; the opponent and the public render must not.
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.current_sector = 1;
        g.peeking = vec![colony_card(), pirate_card()];
        let peeker = brdgme_markup::to_string(&g.player_state(0).render());
        assert!(peeker.contains("Peeked cards"), "got: {peeker}");
        assert!(peeker.contains("Test Colony"), "card identity must be shown");
        let opponent = brdgme_markup::to_string(&g.player_state(1).render());
        assert!(
            !opponent.contains("Peeked cards") && !opponent.contains("Test Colony"),
            "opponent must not see peeked cards"
        );
        let public = brdgme_markup::to_string(&g.pub_state().render());
        assert!(!public.contains("Peeked cards") && !public.contains("Test Colony"));
    }
```

  (`colony_card()` is the existing helper at lib.rs:2161 — its "Test Colony" name appears nowhere else in a fresh game's render; `pirate_card()` was added in Task 2.)

- [ ] Run: `cargo test -p starship-catan-1 sensor_peek`. Expected: FAILS on the first assert — nothing renders the peek today.
- [ ] Implement. In `rust/game/starship-catan-1/src/render.rs`:

  1. Line 108: rename the parameter — `fn render(pub_state: &PubState, player: Option<usize>, peeking: Option<&[SectorCard]>) -> Vec<N> {`.
  2. Immediately after the turn table is pushed (`out.push(table_with_gap(&turn_rows, 2)); out.push(N::text("\n\n"));`, lines 179-180), insert:

```rust
    // Sensor peek: shown only to the player doing the peeking (only the
    // current player can be mid-peek), numbered to match `put <#> top|bottom`.
    if player == Some(pub_state.current_player)
        && let Some(peeking) = peeking
        && !peeking.is_empty()
    {
        out.push(N::Bold(vec![N::text("Peeked cards")]));
        out.push(N::text("\n"));
        let mut peek_rows: Vec<Row> = vec![vec![
            (A::Left, vec![N::Bold(vec![N::text("#")])]),
            (A::Left, vec![N::Bold(vec![N::text("Card")])]),
        ]];
        for (i, c) in peeking.iter().enumerate() {
            peek_rows.push(vec![
                (A::Left, vec![N::text((i + 1).to_string())]),
                (A::Left, c.full_string()),
            ]);
        }
        out.push(table_with_gap(&peek_rows, 2));
        out.push(N::text("\n\n"));
    }
```

  (`Row`, `table_with_gap`, `A` are already imported at render.rs:6; `SectorCard::full_string` is pub.)

- [ ] Run: `cargo test -p starship-catan-1` — new test PASSES, full suite PASSES (in particular `pub_state_does_not_leak_hidden_info` and the two `last_sectors_*` render tests are the regression lock for the untouched paths).
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/render.rs rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): render sensor peek to the peeking player (a F15, WP-13)`

---

### Task 6: "Current turn:" row shows the current player (a F16, minor)

**Problem (restated):** the turn header (`rust/game/starship-catan-1/src/render.rs:123-126`) renders `N::Player(viewer)`:

```rust
    let mut turn_rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Current turn:")])]),
        (A::Left, vec![N::Player(viewer)]),
    ]];
```

Every player (and the public render, viewer = 0) sees their own name as "Current turn:" regardless of whose turn it is. `pub_state.current_player` is already bound as `current` at render.rs:112 and used four lines later for the flight math.

**Fix:** `N::Player(current)` on line 125.

**Edge cases:** viewer == current (their own turn) → unchanged output; ChooseModule phase where BOTH players act — `current_player` is 0 until the first `new_turn`, so the row shows player 0; pre-existing semantics of the field, not worsened by this fix and not in the finding's scope.

**Files:**
- Modify: `rust/game/starship-catan-1/src/render.rs` (line 125)
- Test: `rust/game/starship-catan-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn current_turn_row_shows_current_player_not_viewer() {
        use brdgme_game::Renderer;
        // a F16: with current_player = 1, the pub render (viewer 0) must show
        // player 1 in the Current turn row.
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::ChooseSector;
        g.current_player = 1;
        let rendered = brdgme_markup::to_string(&g.pub_state().render());
        let row = rendered
            .lines()
            .find(|l| l.contains("Current turn:"))
            .expect("Current turn row must render");
        assert!(
            row.contains("{{player 1}}") && !row.contains("{{player 0}}"),
            "got: {row}"
        );
    }
```

  (Asserting on the single rendered LINE is deliberate — `{{player 0}}` and `{{player 1}}` both legitimately appear elsewhere in the output, e.g. the resource-table headers. `N::Player(p)` serializes as `{{player p}}`, same convention the existing `last_sectors_leftmost_bold` test relies on for `{{b}}`.)

- [ ] Run: `cargo test -p starship-catan-1 current_turn_row`. Expected: FAILS — the row contains `{{player 0}}`.
- [ ] Implement: change render.rs:125 from `(A::Left, vec![N::Player(viewer)]),` to `(A::Left, vec![N::Player(current)]),`.
- [ ] Run: `cargo test -p starship-catan-1` — new test PASSES; `last_sectors_leftmost_bold` and `last_sectors_hidden_when_empty` still PASS (they run with current_player = 0 = viewer, so their output is unchanged).
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/render.rs rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): current-turn row shows current player (a F16, WP-13)`

---

### Task 7: direction-mismatch error names the card's direction (a F18, nit)

**Problem (restated):** when a trade's direction is forbidden by the card, `can_trade` (`rust/game/starship-catan-1/src/lib.rs:910-919`) interpolates the ATTEMPTED direction:

```rust
                if trade_dir != TradeDir::Both
                    && direction != TradeDir::Both
                    && trade_dir != direction
                {
                    return (
                        false,
                        0,
                        format!("you can only {} with this trade card", trade_dir.string()),
                    );
                }
```

Trying to buy at a sell-only card yields "you can only buy with this trade card" — the opposite of the truth. The card's allowed `direction` is in scope (destructured at lib.rs:878-893). Reachability nuance (re-derivation note 2): `can_buy`/`can_sell` hit this same check with `Resource::Any` and withhold the verb, so the wrong string never reaches a player through `command()` today — it is still the message every direct `can_trade` caller gets, and the fix is one token.

**Fix:** interpolate `direction.string()`.

**Edge cases:** `direction == TradeDir::Both` → branch not taken (message can't fire); `trade_dir == TradeDir::Both` (amount 0 probe) → branch not taken; the only reachable wording is now "you can only buy ..." on Buy-only cards and "you can only sell ..." on Sell-only cards — both true statements.

**Files:**
- Modify: `rust/game/starship-catan-1/src/lib.rs` (line 917)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test (direct `can_trade` call — see the reachability nuance above):

```rust
    #[test]
    fn direction_mismatch_error_names_card_direction() {
        // a F18: attempting to buy at a sell-only card must say "sell", the
        // card's allowed direction, not echo the attempted "buy".
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.current_sector = 1;
        g.flight_cards = vec![SectorCard::Trade {
            name: "Merchant Outpost".to_string(),
            resources: vec![Resource::Food],
            price: 3,
            maximum: 2,
            direction: TradeDir::Sell,
            trading_post: false,
        }];
        let (ok, _, reason) = g.can_trade(0, Resource::Food, 1); // attempted buy
        assert!(!ok);
        assert_eq!(reason, "you can only sell with this trade card");
    }
```

- [ ] Run: `cargo test -p starship-catan-1 direction_mismatch`. Expected: FAILS — reason reads "you can only buy with this trade card".
- [ ] Implement: change lib.rs:917 from `format!("you can only {} with this trade card", trade_dir.string()),` to `format!("you can only {} with this trade card", direction.string()),`.
- [ ] Run: `cargo test -p starship-catan-1` — full suite PASS.
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): direction error names the card's direction (a F18, WP-13)`

---

### Task 8: cap last_sectors history (a F19, nit)

**Problem (restated):** every flight prepends to `last_sectors` with no cap (`end_flight`, `rust/game/starship-catan-1/src/lib.rs:798-800`), and the renderer prints the entire history in the ChooseSector view (render.rs:129-139). Long games grow the persisted state and the render line without bound for what is recent-history flavor.

**Fix (matches the finding's recommendation):** truncate to the 5 most recent on insert, via a named constant. The field stays `Vec<i32>` — serde shape unchanged; legacy states with longer histories deserialize fine and get trimmed the next time that player's flight ends.

**Edge cases:** fewer than 5 entries → truncate is a no-op; exactly 5 → new entry in, oldest out; the existing render tests use 3 and 0 entries → unaffected; opponent's history untouched by the current player's flight.

**Files:**
- Modify: `rust/game/starship-catan-1/src/lib.rs` (`end_flight`, lines 798-800 + new const)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn last_sectors_capped_on_flight_end() {
        // a F19: history keeps only the most recent entries, newest first.
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.current_sector = 2;
        g.player_boards[0].last_sectors = vec![1, 2, 3, 4, 1];
        g.end_flight();
        assert_eq!(g.player_boards[0].last_sectors, vec![2, 1, 2, 3, 4]);
    }
```

- [ ] Run: `cargo test -p starship-catan-1 last_sectors_capped`. Expected: FAILS — the vec has 6 entries.
- [ ] Implement. In `rust/game/starship-catan-1/src/lib.rs`:

  1. Near the top of the file (after the `use` block, before `Phase`):

```rust
/// Number of recent flight sectors kept per player for the ChooseSector view.
const LAST_SECTORS_LIMIT: usize = 5;
```

  2. In `end_flight`, after the existing insert (lib.rs:798-800), add:

```rust
        self.player_boards[self.current_player]
            .last_sectors
            .truncate(LAST_SECTORS_LIMIT);
```

- [ ] Run: `cargo test -p starship-catan-1` — new test PASSES; `last_sectors_leftmost_bold` (3 entries) and `last_sectors_hidden_when_empty` still PASS.
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/starship-catan-1/src/lib.rs` ; message: `fix(starship-catan-1): cap last_sectors history at 5 (a F19, WP-13)`

---

### Task 9: dead-code removal + flight_actions invariant comment (a F17, minor; a F20 comment-only)

**Problem (restated, a F17):** four items are dead, re-confirmed by crate-wide grep against live source:

1. `Game::next_turn` (`rust/game/starship-catan-1/src/lib.rs:756-759`) — zero callers; `done()` (lib.rs:1495-1496) inlines the identical two lines.
2. `Transaction::gain` (lib.rs:61-69) — zero `.gain()` call sites (`gain_string`/`gain_plain` filter inline; `Transaction::lose` IS used by `can_afford` at lib.rs:333 and stays).
3. `Module::description` + its only caller-of `join_dice` (`rust/game/starship-catan-1/src/card.rs:124-149` and 174-179) — superseded by `Module::summary` (used in render.rs:321); `science_module_dice`/`trade_module_dice` are used by `produce()` and stay.
4. `start_card: bool` field of `SectorCard::Colony` (card.rs:257) — written `false` at both construction sites (card.rs:468 `colony()`, lib.rs:2166 test helper), never read; every `Colony` pattern in the crate matches with `..`.

**Serde compatibility of the field removal (must hold — restated from Architecture):** persisted game states contain `"start_card": false` inside every serialized `Colony` card (in `sector_cards`, `sector_draw_pile`, `flight_cards`, `peeking`, `colonies`). Serde ignores unknown fields on deserialize by default and no type in this crate opts into `deny_unknown_fields`, so old JSON deserializes cleanly into the field-less enum; nothing serializes the field for new states, and no consumer reads it (grep-confirmed, including DATA_DOCS.md — the field is undocumented). If the implementer finds ANY `deny_unknown_fields` or a reader of `start_card` outside the two write sites, STOP and report instead of proceeding.

**a F20 (comment only — change SKIPPED, re-derivation note 4):** `flight_actions: BTreeMap<usize, bool>` (lib.rs:505) only ever stores `true` (sole insert: `mark_card_actioned`, lib.rs:822-824) and both consumers count `true` values (lib.rs:596, 1840). A `BTreeSet<usize>` is the honest type but serializes as a JSON array where the map serializes as an object — deserialization of live mid-flight states would fail. Document instead.

**Files:**
- Modify: `rust/game/starship-catan-1/src/lib.rs`, `rust/game/starship-catan-1/src/card.rs`

**Steps:**

- [ ] In `rust/game/starship-catan-1/src/lib.rs`:
  - Delete `Transaction::gain` (the `pub fn gain(&self) -> Transaction { ... }` block, lines 61-69). Do NOT touch `lose`, `gain_string`, `gain_plain`, or `Game::gain`.
  - Delete `Game::next_turn` (lines 756-759). `done()` keeps its inline equivalent.
  - On the `flight_actions` field (line 505), add the comment:

```rust
    /// Keyed by flight-card index; values are always true (a set in map's
    /// clothing). Kept as BTreeMap<usize, bool> because changing the type
    /// would change the serialized JSON shape and break saved games (a F20).
    pub flight_actions: BTreeMap<usize, bool>,
```

  - In the test helper `colony_card()` (line 2161-2168), delete the `start_card: false,` line.
- [ ] In `rust/game/starship-catan-1/src/card.rs`:
  - Delete `Module::description` (lines 124-149) and `join_dice` (lines 174-179).
  - In `SectorCard::Colony`, delete the `start_card: bool,` field (line 257).
  - In the `colony()` constructor (lines 463-470), delete the `start_card: false,` line.
- [ ] Run: `cargo build -p starship-catan-1` — MUST compile with no missing-field errors (proves every `Colony` construction/pattern site was covered; the compiler is the safety net here).
- [ ] Run: `cargo test -p starship-catan-1` — full suite PASS (including `tests/contract.rs` and `pub_state_does_not_leak_hidden_info`).
- [ ] Serde-compat spot check (one-off, not committed): confirm old-shape JSON still deserializes by running this as a temporary test, then DELETE it before committing (it duplicates what serde guarantees; keeping it would be noise):

```rust
    #[test]
    fn tmp_old_colony_json_still_deserializes() {
        let old = r#"{"Colony":{"name":"X","resource":"Food","dice":1,"start_card":false}}"#;
        let c: SectorCard = serde_json::from_str(old).unwrap();
        assert!(matches!(c, SectorCard::Colony { .. }));
    }
```

  Run `cargo test -p starship-catan-1 tmp_old_colony`, confirm PASS, then delete the test. If it FAILS, revert the `start_card` removal entirely (keep the other three deletions) and record the reversal in the disposition table.
- [ ] `cargo clippy -p starship-catan-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/starship-catan-1/src/lib.rs rust/game/starship-catan-1/src/card.rs` ; message: `refactor(starship-catan-1): remove dead code, document flight_actions shape (a F17, a F20, WP-13)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| a F11 cannon surcharge checks boosters | major | Check `Resource::Cannon` >= 3 | CONFIRMED — fixed (Task 1) | Copy-paste bug verified against live lib.rs:311 and RULES.md:172-173 per-item phrasing; booster sibling locked in by test. |
| a F12 can_lose_module `\|\|` allows voluntary sacrifice | major | Change to `&&` | CONFIRMED — fixed (Task 2) | `losing_module` set only in fight()'s loss branch, cleared only in lose(); `&&` alone closes the exploit (verification's evidence note re-checked live). |
| a F13 TradeAndBuild buys skip astro check | major | Add `amount * price > astro` check | CONFIRMED — fixed (Task 4) | Live trade() debits unconditionally (lib.rs:1065); check added after price lookup, message mirrors the Flight branch. Ordered after Task 3 so the product is overflow-safe. |
| a F14 unbounded amounts overflow i32 | major | checked_mul/checked_add OR parser cap | CONFIRMED — fixed, implementation chosen (Task 3) | Parser cap `Int::bounded(1, 99)` over checked arithmetic: one choke point vs four sites, legal max amount is 4, suggest already caps at min+4. Verification's `maximum: 0` reachability re-confirmed in live card data. |
| a F15 Sensor peek never rendered | major | Render peeked cards, numbered to match `put` | CONFIRMED — fixed, guard added (Task 5) | Rendered as a numbered table via full_string(). ADJUSTMENT over the rec: gated on viewer == current_player, because player_state() serves `peeking` to BOTH players — unconditional rendering would hand the peek to the opponent. JSON-level exposure left to WP-10 (Non-Goals). |
| a F16 Current turn row shows viewer | minor | Use `N::Player(current)` | CONFIRMED — fixed (Task 6) | `current` already bound at render.rs:112; one-token fix, line-scoped render test added. |
| a F17 dead code (next_turn, Transaction::gain, description/join_dice, start_card) | minor | Delete | CONFIRMED — fixed (Task 9) | All four re-confirmed dead by live grep. `start_card` removal is a serialized-shape edit but compatible (serde ignores unknown fields; nothing reads it); one-off deserialization spot check included with a revert instruction if it fails. |
| a F18 direction error echoes attempted direction | nit | Interpolate `direction.string()` | CONFIRMED — fixed (Task 7) | REFINEMENT: the message is unreachable via command() today (can_buy/can_sell gate the verb first), so the test calls can_trade directly. Fix still correct for direct callers. |
| a F19 last_sectors unbounded | nit | Truncate to a small fixed length on insert | CONFIRMED — fixed (Task 8) | Capped at 5 via named const in end_flight; Vec shape unchanged, legacy long histories self-trim on next flight end. |
| a F20 flight_actions BTreeMap only stores true | nit | Switch to BTreeSet, or leave as-is given serialization | SKIPPED (comment only, Task 9) | BTreeMap<usize,bool> = JSON object, BTreeSet<usize> = JSON array: live mid-flight states would fail to deserialize. Dual-shape migration disproportionate for a simplicity nit; the finding and work-packages both sanction leaving it. Invariant documented on the field. |

## Cross-package coordination points

- `player_state()` (lib.rs:1854-1860) exposes `Game::peeking` in the opponent's PlayerState JSON — hidden-info exposure of the WP-10 redaction class (D-33); flagged there, only render-guarded here (Task 5).
- Task 3 relies on lib-game's `Int::bounded` semantics and the suggest cap at suggest.rs:85-96; WP-03's suggest max-cap work does not change this crate's behavior.
- The five duplicated `is_finished()` placings epilogues in `command()` mirror the pattern WP-08 is deduplicating in other crates; starship-catan-1 carries no finding for it, but a future sweep should include this crate.
