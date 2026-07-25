# Verification: starship-catan-1 findings F11-F20

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (f8763a5). Paths relative to rust/.

## F11 — cannon_transaction checks Booster count for science surcharge

Verdict: CONFIRMED

Evidence (game/starship-catan-1/src/lib.rs:307-315):

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

booster_transaction (lib.rs:297-305) checks `self.res(Resource::Booster) >= 3` correctly for boosters. RULES.md:172-173 confirms the intended rule is per-module-type:

```
build booster  # build a booster (2 fuel, plus 1 science once you have 3+)
build cannon   # build a cannon (2 carbon, plus 1 science once you have 3+)
```

The "3+" in each line refers to the item being built. cannon_transaction should check `Resource::Cannon`. Claimed effects (overpay with 3+ boosters / 0-2 cannons; underpay with 3+ cannons / <3 boosters) follow directly.

Severity: major is appropriate — game-rule correctness bug affecting economy in real play.

## F12 — can_lose_module allows current player to `lose` on any pirate

Verdict: CONFIRMED

Evidence (game/starship-catan-1/src/lib.rs:1266-1268):

```rust
pub fn can_lose_module(&self, player: usize) -> bool {
    self.current_player == player || self.losing_module
}
```

For the current player the `||` makes this unconditionally true. Control flow traced:

- command.rs:158-167: when `flight_cards.last()` is a `Pirate` (and `gain_resources.is_none() && !flight_cards.is_empty() && !card_finished`, command.rs:150), the branch pushes `fight_parser()`, `pay_parser()`, and — because `can_lose_module(player)` is true for the current player — `lose_parser()` (command.rs:165-167). So `lose <module>` is offered before any fight, on any pirate including ransom-only ones (`pirate(2, 3, false, false)` etc., card.rs:538-539 have `destroy_module: false`).
- lib.rs:1666-1690: `Game::lose` gates only on `can_lose_module` and module ownership (lib.rs:1672), then decrements the module level, sets `losing_module = false`, and calls `self.end_flight()` (lib.rs:1688). No check that a lost fight occurred.

Contrast: `can_fight` (lib.rs:1242-1248) and `can_pay_ransom` (lib.rs:1250-1257) both require `!self.losing_module` plus phase/pirate checks — `can_lose_module` has none of those guards. `losing_module` is only set true after a lost fight with `destroy_module` (lib.rs:1640-1641). RULES.md documents lose only as the consequence of losing a module-destroying fight (RULES.md ~157-160). So a player can voluntarily destroy a module to skip the pirate and end the flight, bypassing fight/ransom.

Correction to fix note: `&&` alone is the minimal fix but also note `can_lose_module` (unlike siblings) checks no phase and no pirate-card presence; with `&&` those become moot since `losing_module` is only set inside Flight. `&&` is correct.

Severity: major is appropriate — rules violation reachable via a command the parser actively offers.

## F13 — TradeAndBuild buy path never checks astro; trade debits unconditionally

Verdict: CONFIRMED

Evidence: Flight buy branch checks funds (lib.rs:937-947):

```rust
if trade_dir == TradeDir::Buy {
    if amount * price > self.player_boards[player].res(Resource::Astro) {
        return (false, 0, format!("you only have ${}", ...));
```

TradeAndBuild buy branch (lib.rs:996-1011) checks only `can_fit` (lib.rs:999) and `p.buy > 0` (lib.rs:1006-1009), then returns `(true, p.buy, ...)`. No astro comparison anywhere in that branch (lib.rs:976-1038).

`Game::trade` (lib.rs:1064-1066):

```rust
let total = amount * price;
*self.player_boards[player].res_mut(Resource::Astro) -= total;
*self.player_boards[player].res_mut(resource) += amount;
```

Debit is unconditional; with astro 0 and p.buy > 0 the balance goes negative. Reachable: TradeAndBuild phase, player owns a trading post with a buy price (trading_post cards exist, card.rs:540+), issues `buy N <good>` (parser offered when `can_buy`, command.rs:274-276). Note `fit_transaction` clamps only resource maxima/negatives per-resource on the transaction's own keys; Astro is not part of the trade Transaction here — the debit at lib.rs:1065 bypasses transact entirely.

Severity: major is appropriate — correctness: players can spend money they do not have.

## F14 — Int::positive unbounded amount → arithmetic overflow panic (debug)

Verdict: CONFIRMED

Evidence chain:

- command.rs:121 and 136: buy/sell amounts parse via `AfterSpace::new(Int::positive())`.
- lib/game/src/command/parser/mod.rs:88-93: `Int::positive()` is `min: Some(1), max: None` — admits up to i32::MAX.
- Flight path: `trade()` helper builds cards with `maximum: 0` (card.rs:472-481) and several trading_post cards also use maximum 0 (card.rs:540-556), so the lib.rs:921 cap check (`amount != 0 && maximum != 0 && target_amount > maximum`) is skipped for them; then lib.rs:938 computes `amount * price` — with amount = 2147483647 and price >= 2 (e.g. price 3 posts, card.rs:543) this overflows i32, panicking in debug builds.
- TradeAndBuild path: `can_fit` -> `fit_transaction` computes `cur + v` (lib.rs:362, 373); with v = i32::MAX and cur >= 1 this overflows before any rejection.
- lib.rs:1064 `amount * price` is a further overflow site.

In release builds wrapping produces negative values that get rejected downstream (e.g. wrapped total > astro fails, or fit mismatch), matching the claim. Player-reachable debug panic.

Severity: major per stated project policy (player-reachable panic = defect). Appropriate.

## F15 — peeked cards never rendered; _peeking parameter unused

Verdict: CONFIRMED

Evidence (game/starship-catan-1/src/render.rs:64-68, 108):

```rust
impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(&self.peeking))
    }
}
...
fn render(pub_state: &PubState, player: Option<usize>, _peeking: Option<&[SectorCard]>) -> Vec<N> {
```

`_peeking` never appears again in the function body (render.rs:108-354). Logs: sector() logs only "is using the sensor module to peek at N cards" (lib.rs:1377-1388); put logs only "put a card on the {} of the pile" (lib.rs:1352). No log or render path ever shows the identity of the peeked cards, so a human issuing `put <#> top|bottom` (command.rs:209-229) chooses blind. `PlayerState.peeking` (render.rs:55) does carry the data in the JSON, as claimed.

Severity: major is appropriate — the sensor module is effectively unusable through the rendered interface; quality/correctness of the player experience.

## F16 — "Current turn:" renders viewer instead of current player

Verdict: CONFIRMED

Evidence (game/starship-catan-1/src/render.rs:112, 123-126):

```rust
let current = pub_state.current_player;
...
let mut turn_rows: Vec<Row> = vec![vec![
    (A::Left, vec![N::Bold(vec![N::text("Current turn:")])]),
    (A::Left, vec![N::Player(viewer)]),
]];
```

Every viewer sees their own name as the current turn. `current` is bound at render.rs:112 and used for remaining_moves/actions/trades (render.rs:113-118), so the correct value was at hand.

Severity: minor is appropriate — display-only correctness bug, no game-state effect.

## F17 — dead code: next_turn, Transaction::gain, Module::description + join_dice, start_card

Verdict: CONFIRMED (all four items)

- Game::next_turn (lib.rs:756-759): grep across the crate finds no caller; `done()` inlines the identical logic (lib.rs:1495-1496: `self.current_player = (self.current_player + 1) % 2; Ok(self.new_turn())`).
- Transaction::gain (lib.rs:61-69): no `.gain()` call anywhere in game/starship-catan-1/src (grep `\.gain()` returns nothing).
- Module::description (card.rs:124-149) and join_dice (card.rs:174): join_dice is called only from within description (card.rs:142, 146); description itself has no callers. The renderer uses `m.summary()` instead (render.rs:321, defined card.rs:113).
- start_card field (card.rs:257): only ever written `false` (card.rs:468 in the `colony()` helper; lib.rs:2166 in a test); never read — all `Colony { .. }` matches ignore it.

Severity: minor would also be defensible, but as pure dead-code cleanup with zero behavioral effect the review's classification (minor per orig list header says quality; assigned minor) is fine. Note: the finding was filed at minor severity per the batch; appropriate under Simplicity/Quality.

## F18 — direction-mismatch error interpolates attempted direction

Verdict: CONFIRMED

Evidence (game/starship-catan-1/src/lib.rs:910-919):

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

`trade_dir` is the player's attempted direction; buying at a sell-only card yields "you can only buy with this trade card" — the opposite of the truth. `direction` (the card's allowed direction) is destructured in scope at lib.rs:885.

Severity: nit is appropriate — misleading error text only.

## F19 — last_sectors grows unbounded and renders in full

Verdict: CONFIRMED

Evidence (game/starship-catan-1/src/lib.rs:798-800, in end_flight):

```rust
self.player_boards[self.current_player]
    .last_sectors
    .insert(0, self.current_sector);
```

No truncation anywhere (only writers are this insert and initialization to `vec![]` at lib.rs:257). Renderer prints the whole vec (render.rs:129-139): `sectors[0]` bold, then `for s in &sectors[1..]` appends every remaining entry. One i32 per flight, so growth is slow, but the ChooseSector display row widens without limit over a long game and the serialized board grows.

Severity: nit is appropriate.

## F20 — flight_actions: BTreeMap<usize, bool> where values are always true

Verdict: CONFIRMED

Evidence:

- Declaration: `pub flight_actions: BTreeMap<usize, bool>` (lib.rs:505).
- Only production insert: `self.flight_actions.insert(self.flight_cards.len(), true);` (lib.rs:823, mark_card_actioned) — always `true`. (A test also inserts `(0, true), (1, true)`, lib.rs:2449.)
- Consumers just count trues: `self.flight_actions.values().filter(|a| **a).count()` (lib.rs:596 in remaining_actions; lib.rs:1840 for flight_actions_used).

A `BTreeSet<usize>` (or even a counter, since only len-of-trues is consumed) models the data exactly.

Severity: nit is appropriate — Simplicity.
