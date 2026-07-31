# Unit 02 - Game crates: critical + hidden-information fixes

Review of the 2026-07-23 rust-review remediation for the game crates carrying
the criticals and the hidden-information leak class.

- **Commits reviewed** (9): `90dae6d2` WP-10 (pub_state redaction),
  `4e0abe6d` WP-13 (starship-catan-1), `c52f1a53` WP-14 (alhambra-1),
  `52680e57` WP-15 (seven-wonders-1), `7821938a` / `af2c014b` / `b0babb89` /
  `e560a75a` / `6c0c19c4` WP-25 (modern-art-2).
- **Crates**: `rust/game/alhambra-1`, `rust/game/modern-art-2`,
  `rust/game/seven-wonders-1`, `rust/game/starship-catan-1`, plus the
  `pub_state` redaction sweep across `zombie-dice-1`, `for-sale-1` and others.
- **Findings numbered from F-22** (F-01..F-21 belong to Units 01/01b/01c).

- **Files read in full**: `rust/game/modern-art-2/src/{lib.rs,render.rs}`,
  `rust/game/alhambra-1/src/lib.rs`; targeted extracts of
  `rust/game/{zombie-dice-2,for-sale-2,starship-catan-1,seven-wonders-1}/src/`
  covering every `pub_state`/`player_state`/`validate`/`status`, every
  `Log::public` call site, every production `unwrap`/`expect`/`allow`, and each
  WP's named acceptance symbols. Specs recovered from `868094a6`.
- **Findings**: 13 open, F-22..F-35 with F-27 withdrawn (merged into F-35).
  0 Critical, 1 High (F-22), 5 Medium (F-23, F-24, F-28, F-31, F-33),
  7 Low (F-25, F-26, F-29, F-30, F-32, F-34, F-35).

Status: COMPLETE

## Findings

### F-22 (High) - alhambra-1 logs every player's opening hand publicly

`rust/game/alhambra-1/src/lib.rs:160-181` (`Game::start_game`)

```rust
logs.push(Log::public(vec![
    N::Player(p),
    N::text(format!(" drew {}", card_strs.join(", "))),
]));
```

The whole point of `PubBoard.card_count` (`lib.rs:76-77`, doc-commented
"Number of money cards in the player's hand (actual cards are private)") is
that money cards are hidden. The opening deal contradicts it: for every
player, the exact set of cards drawn is emitted as a **public** log. Since
every subsequent hand change is also public (`take` adds named market cards,
`spend` names the cards spent), any observer who reads the log can
reconstruct every player's exact hand for the entire game. The `pub_state`
redaction is therefore cosmetic in this crate.

Why it matters: money-card composition is the core hidden information in
Alhambra - knowing an opponent holds exactly B9 tells you they will win the
blue tile and lets you avoid over-bidding for it. This is precisely the leak
class WP-10 was chartered to close, and the WP-10/WP-14 test
(`pub_state_does_not_leak_hidden_info`, `lib.rs:1366-1374`) only greps the
serialized `PubState` for field names, so it cannot see a log leak.

Remediation: make the opening-deal log private to the drawing player
(`Log::private(..., vec![p])`) plus a public count-only line ("X drew 4
cards"), matching how `PubBoard.card_count` already presents the same
information. Add a test asserting no public log from `start()` contains a
card code belonging to another player.

### F-23 (Medium) - alhambra-1 final-place log publishes each player's private currency totals

`rust/game/alhambra-1/src/lib.rs:452-479` (`Game::final_place_phase`)

```rust
content.push(N::text(format!(
    " had the most money for {} with {} and got {}",
    currency.name(), best_value, tile.tile_type.abbr().trim()
)));
```

`best_value` is `self.boards[p].currency_value(currency)` - a direct
aggregate over the player's private hand - and it is published for the
winning player of each of the 4 currencies. This is the derived-value leak
the unit brief calls out explicitly (an aggregate that reveals hidden
state). It is less damaging than F-22 because it fires at the transition into
`FinalPlace`, but tiles are still placed after this point and the value is
never needed by any other player.

Remediation: log only the winner and the tile; drop `best_value`, or emit it
as a private log to that player.

### F-24 (Medium) - alhambra-1 still has no `validate()` override (F-06 confirmation)

`rust/game/alhambra-1/src/lib.rs:813-963` (`impl Gamer for Game`)

Confirmed per the Unit 01 carry-forward: `alhambra-1` does **not** override
`Gamer::validate`, so the D-36 deserialized-state trust boundary is
fail-open for this crate even after WP-14 hardened its command paths. This
matters more here than in most crates because several code paths index or
`unreachable!()`/`expect()` on state-derived values that a corrupted blob can
violate - e.g. `self.boards[player]` is indexed unchecked in `take`, `spend`,
`place`, `swap`, `remove` and `finish_epilogue`, and `self.tiles[ci]` assumes
`tiles.len() == 4` (`lib.rs:407-417`, `lib.rs:446`, `lib.rs:625`). A
deserialized state with `boards.len() < all_players` or `tiles.len() < 4`
panics the game binary rather than returning `GameError`.

Remediation: implement `validate()` asserting
`MIN_PLAYERS..=MAX_PLAYERS` contains `human_players`,
`all_players == if human_players == 2 { 3 } else { human_players }`,
`boards.len() == all_players`, `tiles.len() == 4`,
`current_player < human_players`, and `1 <= round <= 3`. Compare with
`modern-art-2`'s `validate()` (`rust/game/modern-art-2/src/lib.rs:742-783`),
which is the correct shape.

### F-25 (Low) - alhambra-1 scoring cards are injected at ~20% / ~70% of the deck, not thirds

`rust/game/alhambra-1/src/lib.rs:216-229` (`Game::inject_scoring_cards`)

```rust
let pos1 = fifth + self.rng.random_range(0..fifth);
let pos2 = 3 * fifth + self.rng.random_range(0..fifth) + 1;
```

Positions are counted from the front of `card_pile`, but cards are drawn with
`pop()` from the **back** (`lib.rs:242`). So the first scoring card reached is
`pos2` (~80% of the way from the front = ~20% of the way through play) and the
second is `pos1` (~30% from the front = ~70% through play). Alhambra's rules
put the two scoring cards at roughly the one-third and two-thirds marks. The
`+ 1` on `pos2` is also unexplained.

Why it matters: round 1 scoring fires far too early, when almost nobody has
tiles, so the round-1 reward tier is largely wasted; round 3 (the big tier)
is then compressed into the last 30% of the deck. It skews the game's
scoring balance rather than breaking it, hence Low.

Remediation: index from the draw end, e.g. compute positions as
`len - (2*fifth + rng.random_range(0..fifth))` and
`len - (4*fifth - rng.random_range(0..fifth))`, or reverse the sense of the
existing constants; add a test asserting each scoring card is drawn within
the expected third.

### F-26 (Low) - alhambra-1 final scoring can use the wrong reward tier

`rust/game/alhambra-1/src/lib.rs:377-383` (`Game::next_phase`, `Phase::FinalPlace`)
plus `lib.rs:330-368` (`score_type`)

The final scoring is `self.score_round()`, which scores with `self.round` -
whatever it happens to be. `self.round` only advances when a `DeckCard::Scoring`
is actually drawn. The game ends when the **tile bag** empties
(`lib.rs:407-417`), which is independent of deck progress, so a game that
exhausts tiles before both scoring cards surface performs its final scoring
with the round-1 or round-2 reward slice (`rewards[..round.min(3)]`) instead
of the full three-tier slice. Combined with F-25's mis-positioning this is
reachable more often than it should be.

Remediation: force `self.round = 3` before the final `score_round()` in the
`FinalPlace` -> `End` transition, and note the invariant in a comment.

### F-27 - withdrawn (merged into F-35)

The alhambra-1 `stats: vec![]` observation is filed as F-35, which covers
both affected crates in this unit. Number retired to keep the sequence
stable.

### F-28 (Medium) - modern-art-2's "money is secret" guarantee is fully defeated by public logs

`rust/game/modern-art-2/src/lib.rs:83-86`, `:341-349`, `:430-455`, `:722-732`

`PlayerState.money` is doc-commented "Private until the end of the game",
`PubState` deliberately omits it, `points()` returns all zeros until
`finished` ("cash is secret until the end of the game"), and a dedicated test
`test_pub_state_hides_sealed_bids_and_money` (`lib.rs:1146-1162`) asserts the
redaction. But **every** mutation of `player_money` is published:

- `settle_auction` (`lib.rs:442-450`): `"{winner} bought {cards}, paying ${price} to {seller}"`, a `Log::public`.
- `end_round` (`lib.rs:341-347`): `"Paying {p} ${p_total} for selling all their cards"`, a `Log::public`.
- `end_round` final round (`lib.rs:361-365`): the full money table.

Starting money is the constant `INITIAL_MONEY = 100`. So any client that
replays the public log knows every player's exact balance at all times, and
the redaction is decorative.

Why it matters: secret money is a core mechanic of Modern Art - it is what
makes bidding a judgement call. This is the derived/aggregate leak class the
unit is chartered on, and it is invisible to the existing test because the
test only inspects the serialized `PubState`.

Note: losing **Sealed** bid amounts do stay secret (`bid()`/`pass()` suppress
the log for `Rank::Sealed`, `lib.rs:481-483`, `:525-531`), so that part of
WP-25's work is sound; the leak is the money trail, not the sealed bids.

Remediation: decide the intent explicitly. Either (a) accept money as public
and drop the misleading doc comments, the `points()` zeroing and the test -
this is the honest option and matches the render, which shows no opponent
money; or (b) make the payment logs private to the two parties
(`Log::private`) with a public "X bought Y" line that omits the amount, and
make the end-of-round payout private per player. Do not leave the current
state, where the code claims a guarantee it does not provide.

### F-29 (Low) - modern-art-2 `PubState` omits opponent hand sizes, which are public information

`rust/game/modern-art-2/src/lib.rs:53-75`

`PubState` carries no per-player card count. Hand *contents* are secret in
Modern Art but hand *size* is open information at the table, and the engine
needs it: `advance_past_empty_hands` skips empty-handed players and logs
"Skipping X as they have no cards", so the count is partially leaked anyway
while never being available as data. The renderer's players table
(`render.rs:90-113`) shows purchases only.

Remediation: add `hand_counts: Vec<usize>` to `PubState`, built from
`player_hands.iter().map(Vec::len)`, and show it in the players table. This
is the WP-10 3a canonical shape (counts in public, contents in
`player_state`) which modern-art-2 was never swept for.

### F-30 (Low) - modern-art-2 leaves stale `bids` behind when a round ends mid-auction

`rust/game/modern-art-2/src/lib.rs:304-309` (`end_round`), `:430-455` (`settle_auction`)

WP-25's `af2c014b` "reset auction state when a round ends" clears
`currently_auctioning` and sets `state = PlayCard`, but neither `end_round`
nor `settle_auction` clears `self.bids`. It is currently harmless because
`bids` is reset in `add_card_to_auction` (`lib.rs:422`) before it is next
read, and every reader is gated on `is_auction()`. It is still a broken
invariant: the persisted `Game` carries bid amounts (including secret Sealed
ones) from a concluded auction, and `validate()` does not assert
`state == PlayCard implies bids.is_empty()`.

Remediation: clear `self.bids` alongside `currently_auctioning` in both
`end_round` and `settle_auction`, and add the implication to `validate()`.

### F-31 (Medium) - starship-catan-1 still has no `validate()` override (F-06 confirmation)

`rust/game/starship-catan-1/src/lib.rs` - no `fn validate` anywhere in the crate.

Confirmed per the Unit 01 carry-forward: `starship-catan-1` is one of the 13
crates that never override `Gamer::validate`, so D-36's trust boundary is
fail-open here. WP-13's own spec (Non-Goals) explicitly deferred this to
WP-09 "blocked on D-36", and WP-09b (`c078c3ee`) then covered only 16 files -
this crate was not among them. The exposure is concrete: the crate is
2-player-only with `player_boards: [PlayerBoard; 2]`, `current_player` is
used as an index throughout, and `player_state`/`pub_state` index it
directly, so a deserialized state with `current_player > 1` panics.

Remediation: implement `validate()` asserting `current_player < 2`,
`1 <= current_sector <= 4`, and that `sector_cards` has exactly the keys
1..=4. This is the smallest change that closes the panic surface.

### F-32 (Low) - starship-catan-1 clones both full `PlayerBoard`s into `PubState`, contrary to WP-10's own heuristic

`rust/game/starship-catan-1/src/render.rs:22-23`, `rust/game/starship-catan-1/src/lib.rs:1851`

```rust
player_boards: self.player_boards.clone(),
```

WP-10 section 3a rule 5 states the heuristic plainly: "if a field is an
ordered deck or a `Vec` indexed by player, it may not be cloned into
`PubState`". `player_boards` is exactly a per-player array cloned straight
through, and rule 1 forbids clone-through of a private `Game` field. In
Starship Catan the board (resources, modules, colonies, trading posts) is
genuinely open information, so there is no leak today - but nothing in the
crate records that judgement, and the redaction test
(`pub_state_does_not_leak_hidden_info`, `lib.rs:2221-2252`) only blocklists
four field names, so any future private field added to `PlayerBoard` becomes
public silently and no test fails.

Remediation: no code change required, but add a comment on the
`player_boards` field stating that every `PlayerBoard` field is open
information by design and that private per-player state must go in
`PlayerState`; and change the test from a blocklist of four names to an
allowlist of the expected `PubState` key set so a new field forces a
deliberate decision.

### F-33 (Medium) - seven-wonders-1 also has no `validate()`; the F-06 crate list under-counts this unit

`rust/game/seven-wonders-1/src/lib.rs` - no `fn validate` anywhere in the crate.

The Unit 01 carry-forward named `alhambra-1` and `starship-catan-1` as this
unit's members of the 13 non-overriding crates. `seven-wonders-1` is a third:
`impl Gamer for Game` has no `validate()`. WP-15's Non-Goals deferred it to
WP-09 ("blocked on D-36"), and WP-09b never reached it either.

This one is the worst of the three because the crate indexes by player index
in hot paths without any `get()`: `status()` reads `self.hands[p]`,
`self.actions[p]` and `self.coins[p]` for `p in 0..self.players`
(`lib.rs:894-925`), and `end_hand` does `self.hands[p].pop().unwrap()`
(`lib.rs:211`). A saved state where `hands.len() < players` - or where
`actions` and `hands` disagree in length - panics inside `status()`, which the
web layer calls on every page render, not just on a command. That turns a
corrupted blob into an unrecoverable game rather than a `GameError`.

Remediation: implement `validate()` asserting `3 <= players <= 7`,
`1 <= round <= 3`, and that `hands`, `actions`, `cards`, `coins`,
`victory_tokens`, `defeat_tokens` and `cities` all have length `players`, plus
`to_resolve`'s player indices are `< players`.

### F-34 (Low) - seven-wonders-1's new "no takeable cards" log leaks a property of the hidden discard pile

`rust/game/seven-wonders-1/src/lib.rs:722-725`

```rust
logs.push(Log::public(vec![
    N::Player(player),
    N::text(" has no cards they can take from the discard pile"),
]));
```

This line is new with WP-15's b F2 prune fix. `PubState` deliberately exposes
the discard pile as `discard_count` only (`lib.rs:83-84`), so the pile's
contents are hidden. The message states that **every** card in a pile of
known size is already built by a player whose tableau is fully public
(`PubState.cards`), which lets any observer narrow the pile's contents
substantially - the more so late in an age when `discard_count` is small.

It is Low rather than Medium because b F8 ("discard pile hidden from
players") is a **parked** rules-adjudication item whose likely resolution is
to make the pile public anyway, so this leak may become moot. Flagging it so
the parked decision is made deliberately rather than pre-empted by a log
line.

Remediation: emit it as a private log to `player` (they are the only one who
needs to know why their resolver vanished), or defer until b F8 is
adjudicated. Do not leave a public log asserting a fact about hidden state.

### F-35 (Low) - `Status::Finished { stats: vec![] }` in two crates in this unit

`rust/game/alhambra-1/src/lib.rs:918-921`, `rust/game/seven-wonders-1/src/lib.rs:900-903`

Both return a zero-length `stats` alongside a `placings` of length
`players`. Every other reviewed crate returns
`vec![HashMap::new(); players]` (e.g.
`rust/game/modern-art-2/src/lib.rs:617-620`). Any consumer that zips or
indexes `stats` by player either silently drops everyone or panics. Related
to but distinct from F-19 (which is about placings being computed twice).

Remediation: `stats: vec![HashMap::new(); self.players]` in both, or - better
- have `lib/game` provide a constructor that cannot get the two lengths out
of step.

## Verified good

### WP-10 (`90dae6d2`) - pub_state hidden-info redaction: fully implemented as specified

All three items in the spec's section 3 land exactly as written, and the
exploits are closed rather than papered over:

- **zombie-dice-2 (f F1, cup draw order).** `PubState::cup` is gone;
  `pub_state()` (`rust/game/zombie-dice-2/src/lib.rs:496-512`) *constructs*
  `cup_counts` as a fixed-order Green/Yellow/Red triple including zeros.
  `Game::cup` itself is untouched, so the draw order still exists internally
  but is not recoverable from the public view - composition is public, order
  is not, which is the correct line. `render_cup`
  (`rust/game/zombie-dice-2/src/render.rs:51-69`) takes the counts, skips
  zeros and renders grey "None" when empty, so no UI regression. The existing
  `test_pub_state_captures_rendered_fields` (`lib.rs:1028-1052`) was
  *strengthened* rather than weakened: it now derives the expected triple
  from `g.cup` instead of asserting `g.cup == ps.cup`. I checked for a
  side-channel: `current_roll` and `kept` are cloned through, but both are
  face-up dice, and `take_dice` (`lib.rs:239-255`) reshuffles via
  `shake_cup()` on refill, so no residual ordering signal.
- **for-sale-2 (f F13, selling-phase bids).** `pub_state()`
  (`lib.rs:449-453`) emits `vec![0; self.players]` during `Phase::Selling`
  and clones as before in Buying, `PlayerState.bid` (`lib.rs:85-86`) carries
  the viewer's own play, and the `Phase::Selling` render arm
  (`render.rs:105-113`) reads `own.bid`, never `pub_state.bids`.
  `finished_bidding` stays public per the spec's explicit ruling. Two new
  tests pin both directions (`test_selling_phase_redacts_bids`
  `lib.rs:883-902`, `test_buying_phase_bids_are_public` `lib.rs:904-913`) and
  the pre-existing `test_pub_state_redacts_hands_and_cheques` still asserts
  `g.bids == ps.bids` at game start (Buying) unmodified. The public reveal
  log in `play()` (`lib.rs:274-288`) fires only once `whose_turn_inner()` is
  empty, i.e. after every player has played, so the reveal ordering is
  correct.
- **starship-catan-1 (routed-in WP-13 non-goal, Sensor peek).**
  `player_state()` (`lib.rs:1870-1880`) returns `self.peeking.clone()` only
  when `player == self.current_player`, `vec![]` otherwise - exactly section
  3d, "nothing else in the crate changes". Two tests cover it at both levels:
  `peeking_only_visible_to_current_player` (`lib.rs:2559-2570`) on the data,
  and `sensor_peek_rendered_only_to_peeking_player` (`lib.rs:2536-2557`) on
  the markup, including a negative assertion on `pub_state().render()`. The
  WP-13 Task 5 render gate and the WP-10 data gate are therefore consistent
  and neither widened the other.

No `#[allow]`, `todo!()`, `unwrap()` in a fallible path or weakened test was
introduced by this commit.

### WP-25 (`7821938a`, `af2c014b`, `b0babb89`, `e560a75a`, `6c0c19c4`) - modern-art-2 liveness

Each of the six code findings is closed in the end state, with a regression
test that would actually fail on regression:

- **d F34 (critical infinite busy-loop in `settle_auction`).** Closed by
  `advance_past_empty_hands` (`rust/game/modern-art-2/src/lib.rs:457-471`),
  which checks `all(|h| h.is_empty())` *before* entering the `while`, so the
  loop can only run when at least one hand is non-empty and therefore always
  terminates. The shared invariant is used from both `settle_auction`
  (`:453`) and the round-advance path in `end_round` (`:371`), which is the
  single-invariant shape the spec asked for. The recursion
  `end_round -> start_round -> advance_past_empty_hands -> end_round` is
  bounded by `ROUNDS`, terminating at `round == ROUNDS - 1` where `finished`
  is set.
- **d F35 (round-4 soft-lock).** `end_round`'s advance path now calls
  `advance_past_empty_hands` after `start_round` (`:370-371`), which round 4
  needs because `round_cards` deals 0 cards. Covered by
  `round_four_skips_empty_handed_starter` and
  `round_four_with_no_cards_anywhere_ends_immediately`.
- **Epilogue hoist.** `command()` emits the `placings_log` once after the
  match (`:705-710`), covering the new `Pass`/`Bid`/`Buy` finish paths. Unit
  01c already ruled this in scope for WP-25 and verified it.
- **d F41 (stale `State::Auction` on game end).** `end_round` clears
  `currently_auctioning` and sets `State::PlayCard` (`:308-309`); pinned by
  `game_end_via_stale_auction`-style assertions in
  `game_end_via_fifth_card_leaves_no_stale_auction` (`:1302-1321`), which
  checks `is_auction`, `auctioning`, `auction_type` and `current_bid`
  together. Incomplete only in that `bids` is not cleared - see F-30.
- **d F42 ("$0 by auctioneer" render line).** Gated at both layers: the data
  layer suppresses `current_bid` for Sealed (`:638-642`) and the render layer
  additionally requires `bid > 0` (`render.rs:62-65`). `no_current_bid_line_before_any_bid`
  (`:1354-1379`) asserts both the absence before a bid and the presence
  after, so the fix cannot be regressed into a blanket suppression.
- **d F44/F45/F46 nits.** `settle_auction` uses `std::mem::take` for the
  auction cards (`:433`), no `unwrap()` remains outside tests, and the
  parked items were respected: `end_round`'s ranking loop (`:310-334`), its
  payment loop (`:336-349`) and `highest_bidder`'s tie behaviour (`:189-201`)
  are untouched, so d F36/F37/F43 stay parked as decided (I deliberately did
  not re-raise the zero-card-artist ranking).

Sealed-bid secrecy specifically: `bid()` and `pass()` suppress the public log
for `Rank::Sealed` (`:481-483`, `:525-531`), and `min_bid()` returns 1 for
Sealed so no highest-bid value is needed by the parser. Losing sealed bids
are genuinely never published. (The money trail is a separate problem - F-28.)

Test-quality note, not a finding: `all_hands_empty_after_settle_ends_the_game`
(`:1168-1212`) drives the busy-loop repro on a spawned thread with a 2s
`recv_timeout`. It is the right way to make an infinite loop fail a test
suite, but on regression it leaks a spinning thread for the rest of the run.
Acceptable; worth knowing if the suite ever starts hanging.

### WP-13 (`4e0abe6d`) - starship-catan-1: all nine items land

Verified against the spec's task list, symbol by symbol:

- **a F11 cannon surcharge.** `cannon_transaction`
  (`rust/game/starship-catan-1/src/lib.rs:300-308`) now tests
  `self.res(Resource::Cannon) >= 3`; `booster_transaction` (`:290-298`) still
  tests `Booster`. Both directions pinned by
  `cannon_surcharge_keys_off_cannons_not_boosters` (`:2409`) and
  `booster_surcharge_keys_off_boosters` (`:2430`) - the sibling test is the
  right call, it stops a symmetric copy-paste recurring.
- **a F12 `can_lose_module`.** `lib.rs:1270-1272` is now
  `self.current_player == player && self.losing_module` - the `||` is gone, so
  a player can no longer volunteer a module to skip a pirate. Covered both
  ways (`lose_rejected_without_lost_fight` `:2445`,
  `lose_works_after_losing_module_fight` `:2462`).
- **a F13 astro affordability.** `can_trade`'s `Phase::TradeAndBuild` /
  `TradeDir::Buy` branch checks `amount * p.buy > res(Astro)`
  (`lib.rs:1003-1012`) in addition to the pre-existing `Phase::Flight` check
  (`:931-941`), so the direct `res_mut` debit in `trade` can no longer drive
  astro negative. Boundary covered by `trade_and_build_buy_requires_astro`
  (`:2506`) and `trade_and_build_buy_allows_exact_astro` (`:2524`) - the
  exact-astro test is the one that matters and it is present.
- **a F14 i32 overflow.** `command.rs:128` and `:143` use
  `Int::bounded(1, MAX_TRADE_AMOUNT)` for `buy`/`sell`, and a named constant
  was used rather than the spec's literal 99. Pinned by
  `huge_buy_amount_rejected_before_arithmetic` (`:2482`). `complete` and `put`
  still use `Int::positive()` (`command.rs:112`, `:224`), which the spec
  scoped out; both are index selectors validated downstream, and neither
  feeds a multiplication.
- **a F15 / WP-10 3d Sensor peek.** Covered under WP-10 above; the render gate
  and the data gate are consistent.
- **a F16 current-turn row.** `render.rs:122-126` uses `N::Player(current)`,
  not the viewer. Pinned by `current_turn_row_shows_current_player_not_viewer`
  (`:2573`). The "Last sectors" row still reads `boards[viewer]`
  (`render.rs:129-130`), which the spec explicitly declared out of scope.
- **a F17 dead code.** `start_card` is gone from the whole crate (`rg` finds
  zero occurrences), and the serde-compatibility reasoning for removing it
  holds - no `deny_unknown_fields` anywhere.
- **a F18 direction error.** `can_trade:908-912` now formats
  `direction.string()` (the card's direction) rather than the attempted one.
  Pinned by `direction_mismatch_error_names_card_direction` (`:2590`), which
  correctly calls `can_trade` directly since the message is unreachable via
  `command()`.
- **a F19 `last_sectors` cap.** `end_flight` inserts at 0 then
  `truncate(LAST_SECTORS_LIMIT)` (`lib.rs:789-794`). Pinned by
  `last_sectors_capped_on_flight_end` (`:2609`), which asserts the exact
  resulting vector including the drop of the oldest entry.
- **a F20 `flight_actions` BTreeMap.** Correctly skipped; the serde-shape
  argument in the spec is sound (map-vs-array JSON), and `pub_state` derives
  `flight_actions_used` as a count (`lib.rs:1856`) rather than exposing the
  map, which is the right shape anyway.

Production `unwrap()` audit for this crate: `lib.rs:767`, `:769`, `:803`,
`:1222`, `:1537`, `:1555`, `:1632`. Each is dominated by a guard in the same
or the calling function - `next_sector_card` returns early on `pile_empty`
(`:756-762`), `replace_card` checks `!sector_draw_pile.is_empty()` (`:802`),
`can_end` checks `flight_cards.is_empty()` (`:1214`), and the `found_*`/
`fight` pops are gated by `can_found_*`/`can_fight`, which all require
`!flight_cards.is_empty()`. They are locally provable, so not findings, but
`let Some(x) = .. else` would remove the proof burden.

### WP-14 (`c52f1a53`) - alhambra-1: the duplicate-card-mint critical is genuinely closed

- **b F16 duplicate mint (the critical).** `take` (`lib.rs:564-604`) now uses
  clone-and-verify: it removes each requested card from a **copy** of the
  market, so `take b1 b1` against a single market B1 fails on the second
  lookup, and `self.cards = market` commits only after every check passes. The
  bare `contains()` pre-check is gone. Crucially the fix preserves legitimate
  duplicates (the deck holds 2-3 copies of each card), and both halves are
  tested: `take_cannot_mint_duplicate_cards` (`:1533-1555`) also asserts the
  market **and** the hand are unchanged after the failed take - i.e. no
  partial mutation - and `take_allows_real_duplicates_in_market`
  (`:1558-1579`) stops the fix being over-tightened into "no duplicates
  ever". `spend` (`:606-671`) uses the same shape against the hand. This is
  the right fix, not a symptom patch.
- **b F17 place/swap index.** `nth_non_empty` (`:103-110`) maps the
  renderer's 1-based non-Empty numbering to a raw index, and `place`/`swap`
  both route through it (`:682-690`, `:729-730`). Three tests cover the
  interesting cases: index stability after a placement
  (`place_index_matches_rendered_index_after_placement` `:1582`), rejection
  of a now-out-of-range index (`:1611`), and Empty sentinels in `reserve`
  from legacy corrupted states (`:1626`). The last one is hardening the fix
  did not strictly need, and it is the right instinct.
- **b F18 wall walk.** `test_grid_longest_ext_wall_diagonal_blocker`
  (`:1306-1324`) is constructed so both walk directions hit the truncating
  candidate first, so it fails on the old code from either start segment -
  a genuinely discriminating regression test rather than a happy-path one.
- Nits: logs use `abbr()`/`Currency::name()` rather than Debug formatting,
  pinned by `logs_use_display_names_not_debug` (`:1804-1833`) which checks
  both a success log and an error string. The two `.expect()` calls
  (`:445`, `:624`) are on `Currency::ALL.iter().position(..)` for a value that
  came from `Currency::ALL`, and `command.rs:143`'s `.expect()` is on
  `Card::parse` of a string `Enum::exact` generated from a `Card` - all three
  are provably infallible and carry a justifying message.

F-22, F-25 and the alhambra half of F-35 are all **pre-existing**, not
regressions: `git show c52f1a53^:rust/game/alhambra-1/src/lib.rs` contains the
same `" drew {}"` public log, the same `inject_scoring_cards`, and the same
`stats: vec![]`. WP-14 was scoped to the mint/index/wall items and did not
claim them. They are coverage gaps in the programme, not failures of this
commit.

### WP-15 (`52680e57`) - seven-wonders-1: all items land, and b F2's fix is better than the finding's suggestion

- **b F1 Halicarnassus VP.** `player_vp` (now
  `rust/game/seven-wonders-1/src/scoring.rs:59-84`) has the arm
  `CardEffect::DrawDiscard { vp: stage_vp } => vp += stage_vp` at `:78`,
  binding a distinct name as the spec's re-derivation required.
- **b F2 DrawDiscard soft-lock (the reachable permanent lock).**
  `prune_resolvers` (`lib.rs:714-728`) loops while the head resolver's player
  has no takeable discard, removing one resolver per iteration - so it always
  terminates - and it is called at **both** choke points the spec identified:
  after `execute_actions` in `check_hand_complete` (`:271`) and after each
  `take` removes a resolver (`:755`). The queue-time guard
  `!self.discard.is_empty()` was correctly left in place (`:438`) with a
  comment (`:434-437`) explaining why queue-time filtering is the wrong
  choke point. I checked the multi-resolver case the spec worried about: the
  second resolver is re-pruned after the first player's take, and
  `lib.rs:1343-1346` asserts exactly that. This is a case where the
  implementer overruled the finding's recommendation and was right to.
- **b F3 auto-discard coins.** `end_hand`'s `max_hand == 1` branch
  (`lib.rs:207-226`) pushes the card to `self.discard` with no coin credit;
  the log text ("discarded their last card") never mentioned coins, so it is
  now simply accurate, as the spec predicted.
- **b F9 stored trade deal.** `Action::Build` keeps `deal: Option<usize>` and
  adds `#[serde(default)] deal_coins: Option<HashMap<i32, i32>>`
  (`lib.rs:35-45`) with doc comments recording *why* both exist.
  `resolve_deal` (`trade.rs:80-95`) prefers `deal_coins` and falls back to
  the legacy recompute-and-index path only when it is `None`, marked "Legacy
  fallback for pre-upgrade pending actions only". New writes always set
  `deal: None` + `deal_coins: Some(..)` (`lib.rs:646-647`, `:695-696`). Both
  halves are tested: `legacy_action_json_still_deserializes` (`:1350-1366`)
  round-trips a real pre-upgrade JSON blob, and
  `stored_deal_paid_despite_mid_turn_divergence` (`:1368-`) deliberately
  sabotages the neighbour's goods after the choose so that a recompute
  *cannot* reproduce the deal - that is the correct way to prove the stored
  value is the one paid.
- **b F10 index guard, b F12 military log, b F13 dead code.**
  `player_state` uses `self.hands.get(player).cloned().unwrap_or_default()`
  (`:817`); the military log uses `N::Player(right)` (`:564-572`) and
  `military_log_uses_player_node` (`:1417-1429`) asserts the exact rendered
  markup string; `start_hand` no longer exists anywhere in the crate.
- **b F15 module split.** `scoring.rs` and `trade.rs` are real extractions
  with `pub(crate)` boundaries, not a copy. `scoring.rs:39`'s
  `counts.get_mut(&field).unwrap()` is dominated by the `entry().or_insert(0)`
  two lines above (`:37`), so it is safe.
- Parked items were respected: nothing in the crate exposes discard-pile
  contents (`PubState.discard_count` only), so b F8 stays parked - see F-34
  for the one place a log gets close to it.

### Redaction shape, per crate (WP-10 section 3a compliance)

| Crate | `pub_state` shape | Verdict |
|---|---|---|
| zombie-dice-2 | `cup_counts` constructed from `cup` | Compliant |
| for-sale-2 | `bids` zeroed in Selling, own bid in `PlayerState` | Compliant |
| starship-catan-1 | deck/pile lengths only; `peeking` gated | Compliant on decks; `player_boards` clone-through (F-32) |
| seven-wonders-1 | `discard_count`, `hand_sizes`, `actions_chosen` | Compliant - the best example in the unit |
| modern-art-2 | hands/money/deck omitted | Data-layer compliant, defeated by logs (F-28); missing hand counts (F-29) |
| alhambra-1 | `card_count`, `tile_bag_len` | Data-layer compliant, defeated by logs (F-22, F-23) |

## Coverage gaps

1. **WP-10 was never swept across the remaining game crates, and no WP
   claimed the sweep.** WP-10's own text says it "decides the redaction shape
   once for every game crate. Later crates copy section 3a verbatim" - but it
   only *implemented* three crates, and no subsequent work package audited
   the other 24. F-22, F-23, F-28 and F-29 are all direct consequences in
   this unit's four crates alone. Units 03/04 should treat "does this crate's
   `pub_state` and its **public logs** satisfy WP-10 3a" as a standing check
   for every crate they touch, not just where a finding exists.
2. **No crate in this unit tests hidden information at the log layer.** Every
   redaction test greps the serialized `PubState`
   (`pub_state_does_not_leak_hidden_info` in alhambra-1 and starship-catan-1,
   `test_pub_state_hides_sealed_bids_and_money` in modern-art-2). None
   asserts anything about `Log::public` content. That is precisely why F-22
   and F-28 survived a remediation programme aimed at this bug class. A
   shared test helper in `lib/game` - "no public log emitted by this
   command may contain a token from another player's private state" - would
   have caught both.
3. **`validate()` is missing in three of this unit's four crates**
   (F-24, F-31, F-33), and the F-06 crate list under-counted this unit. The
   next Lead to touch F-06 should re-derive the list mechanically rather than
   trusting the recorded count.
4. **Not reviewed by me:** `alhambra-1/src/card.rs` (88 lines changed by
   WP-14: `Card::parse`, `grid_is_valid`, `grid_longest_ext_wall` internals) -
   I verified the wall-walk fix through its test rather than by reading the
   algorithm. `seven-wonders-1/src/card.rs` (the card database, declared
   read-only by the spec). `starship-catan-1`'s `fight`/`complete`/adventure
   resolution beyond the specific WP-13 items. `modern-art-2/src/command.rs`
   and `alhambra-1/src/command.rs` parsers beyond the `Int` bounds and the
   `Enum::exact` expect.
5. **Possible unverified panic in seven-wonders-1:** `execute_build`'s wonder
   path indexes `city.wonder_stages[stages_built]` (`lib.rs:340`) where
   `stages_built` counts the player's `CardKind::Wonder` cards. I did not
   read the `can_build_wonder` guard, so I cannot say whether a 4th stage on
   a 3-stage wonder is rejected before this index. Worth one grep by whoever
   picks up F-33.

## Carry-forwards for Units 03/04

- Check public **log** content for hidden-info leaks, not just `pub_state`
  fields. This is the single highest-yield check in the game crates and the
  remediation programme did not perform it.
- Re-derive the F-06 non-overriding-`validate` crate list from the tree; the
  recorded list is incomplete.
- `Status::Finished { stats: vec![] }` (F-35) is likely present in more
  crates than the two here - worth a one-line `rg`.
- Do not re-raise: modern-art-2's `end_round` ranking/payment loops and
  `highest_bidder` ties (d F36/F37/F43, parked under D-26/D-30/D-32);
  seven-wonders-1's discard-pile visibility (b F8, parked under D-27/D-28);
  starship-catan-1's `flight_actions` BTreeMap shape (a F20, deliberately
  skipped for serde compatibility) and the "Last sectors" viewer row.
