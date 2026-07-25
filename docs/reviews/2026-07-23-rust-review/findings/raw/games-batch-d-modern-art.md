# Raw findings: game/modern-art-2 (Worker, read-only review)

Snapshot reviewed: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/modern-art-2/`.
Line numbers are per that snapshot. Go port source (`brdgme-go/modern_art_1/modern_art.go`)
was present in the snapshot and used for port-parity comparison; where a finding is judged
against official Modern Art (Knizia) rules rather than the Go port, this is stated explicitly.

Files read in full: `Cargo.toml`, `src/lib.rs`, `src/card.rs`, `src/command.rs`,
`src/render.rs`, `RULES.md`, `tests/contract.rs`. The 4 binaries were skimmed only.

### Infinite busy-loop (hang + unbounded log growth) when all hands empty after a settle
- severity: critical
- category: correctness
- location: game/modern-art-2/src/lib.rs:452
- finding: `settle_auction` advances `current_player` and then loops
  `while self.player_hands[self.current_player].is_empty() { ...; self.next_player(); }`
  with no guard for the case where *every* player has an empty hand. In that case the loop
  never terminates: it spins forever, pushing a "Skipping ..." log entry every iteration
  (unbounded memory growth → worker hang/OOM). Reachable via legal play: the round only ends
  when a 5th card of an artist is played (`add_card_to_auction`, lib.rs:423). In round 4 no
  cards are dealt, and it is mathematically possible for players to collectively hold ≤20
  cards entering round 4 (e.g. 3 players, ≥46 cards played in rounds 1–3) and to play them
  all with at most 4 per artist (no 5-card trigger). When the last auction settles, all
  hands are empty and the loop spins. Judged against official rules the game is also simply
  stuck in this state (no end-of-round trigger), so a fix must decide the round ends here.
  The identical loop exists in the Go port (`modern_art.go:690`), so this is port-inherited.
- recommendation: Before/inside the skip loop, check whether all players' hands are empty;
  if so call `end_round()` instead of looping (or bound the loop to `self.players`
  iterations and break into `end_round()` when a full cycle completes).

### Round 4 can start on an empty-handed player, soft-locking the game
- severity: major
- category: correctness
- location: game/modern-art-2/src/lib.rs:368
- finding: `end_round` does `self.round += 1; self.next_player(); logs.extend(self.start_round());`.
  Round 4 deals 0 cards (`round_cards` returns 0), so if the player after the round-ending
  player has an empty hand (they played out their hand in round 3 — hands persist across
  rounds), the game enters `State::PlayCard` with a `current_player` who has no cards.
  `can_play` is true for them but no parser can produce a card, `pass` is unavailable in
  PlayCard state, and `whose_turn_players` returns only that player — the game is deadlocked
  with no legal command for anyone. The empty-hand skip logic exists only in
  `settle_auction`, not on the round-transition path. Same structure in the Go port
  (`modern_art.go:432-434`).
- recommendation: After `next_player()` in `end_round` (or at the end of `start_round`),
  skip players with empty hands the same way `settle_auction` does; combined with the fix
  for the all-hands-empty case above, this should terminate in `end_round` when nobody can
  play.

### End-of-round payout pays cumulative value for ALL purchased cards, including non-top-3 artists
- severity: major
- category: correctness
- location: game/modern-art-2/src/lib.rs:336
- finding: Judged against official Modern Art rules, not the Go port. The payout loop pays
  `self.suit_value(c.suit)` (the artist's cumulative cross-round value) for every card in
  `player_purchases`. Official rules: only paintings of the three artists that placed this
  season pay out; paintings of the other two artists are worthless that season, even if the
  artist earned value in earlier seasons (CMON rulebook: "If artist is in TOP 3, add all
  previous round's values"; BGG consensus: "only art in top 3 ... is worth something and
  remaining is worthless"). Concretely: Lite Metal places 1st in round 1 (+30); in round 2
  it does not place — official rules pay $0 for Lite Metal cards bought in round 2, this
  implementation pays $30 each. Materially changes game economy and strategy. The Go port
  has identical behavior (`modern_art.go:406-415`), and `RULES.md:103-104` documents the
  implemented behavior, so this is a port-inherited rules deviation the Lead must adjudicate
  (fix, or document as a deliberate house rule).
- recommendation: Either pay only for cards whose suit is in the current round's `values`
  map (top 3), or explicitly document the deviation as intentional (in-code comment +
  RULES.md "house rule" note).

### Artists with zero cards played are ranked and awarded $20/$10
- severity: major
- category: correctness
- location: game/modern-art-2/src/lib.rs:318
- finding: Judged against official Modern Art rules, not the Go port. In `end_round` the
  ranking loop initialises `highest_count = -1`, so artists with 0 cards on the table are
  still selected for 2nd ($20) and 3rd ($10) place when fewer than 3 artists had cards
  played (common: a fast round ending 5-0-0-0-0). Official rules only rank artists that
  actually had paintings played ("only art in top 3 *if there is 3 art out*"). The awarded
  values land in `value_board` and inflate those artists' cumulative values for all later
  rounds. Go port identical (`modern_art.go:389-403`).
- recommendation: Skip candidates with `counts[&s] == 0` (start `highest_count` at 0 and
  require strictly greater, or filter zero counts); leave unplaced artists out of the round's
  `values` map as today. If the deviation is deliberate, document it.

### `unreachable!()` and unchecked indexing in `round_cards`
- severity: minor
- category: quality
- location: game/modern-art-2/src/lib.rs:95
- finding: `round_cards` hits `unreachable!()` for any `players` outside 3..=5 and indexes
  `table[round]` unchecked (panics for round > 3). Both are guarded in practice
  (`Game::start` validates player count; `end_round` finishes at `ROUNDS - 1`), so this is
  only reachable via a corrupt/deserialized state, but repo rules forbid
  `unreachable!()`/`panic!` in runtime paths and `Game` is `Deserialize`, so a defensive
  fallback is cheap.
- recommendation: Replace the match with a saturating/default arm (e.g. `.get(round).copied().unwrap_or(0)`
  and a default table for unexpected player counts) or return a `GameError::internal`.

### RULES.md says the auction winner takes the next turn; implementation (and official rules) pass clockwise from the seller
- severity: minor
- category: consistency
- location: game/modern-art-2/RULES.md:76
- finding: "The winner adds the card(s) to their purchases ... and it becomes their turn
  next" reads as the winner becoming the next auctioneer. The implementation
  (`settle_auction` → `next_player()`, lib.rs:451) passes the turn clockwise from the
  seller, which matches official Modern Art rules ("the player to the left of the seller
  auctions next") and the Go port. Only the doc is wrong/misleading.
- recommendation: Reword to "and the player to the seller's left auctions next".

### RULES.md Double-auction section omits Once Around as a valid added card
- severity: minor
- category: consistency
- location: game/modern-art-2/RULES.md:63
- finding: "Double - Works like Open, Fixed Price, or Sealed depending on the second card
  added" — but `add_card` (lib.rs:396) only rejects another Double, so a Once Around card
  can also be added (and the auction then runs as Once Around). Official rules allow any
  non-Double card of the same artist; the implementation is correct and the doc is
  incomplete.
- recommendation: Add "Once Around" to the list in RULES.md.

### Game-ending mid-auction leaves `state = State::Auction`, so final pub state/render shows a stale auction
- severity: minor
- category: correctness
- location: game/modern-art-2/src/lib.rs:366
- finding: When the game ends (5th card played in round 4), `end_round` sets
  `self.finished = true` but never resets `self.state`, which is still `State::Auction`
  from `add_card_to_auction`. `whose_turn_players`/`status` short-circuit on `finished`,
  so there is no functional deadlock, but `pub_state().is_auction` stays true and
  `Renderer for PubState` (render.rs:57) prints "<player> is auctioning <cards>" with an
  empty `auctioning` vec on the final game screen.
- recommendation: Set `self.state = State::PlayCard` (and ensure `currently_auctioning` is
  cleared — it already is) in the `finished` branch of `end_round`.

### "Current bid: $0 by <auctioneer>" rendered before anyone has bid
- severity: nit
- category: consistency
- location: game/modern-art-2/src/render.rs:62
- finding: For any non-Sealed auction, `pub_state` sets `current_bid = Some(self.highest_bidder())`
  (lib.rs:628) and `highest_bidder` returns `(auctioneer, 0)` when no bids exist, so the
  render shows "Current bid: $0 by <auctioneer>" even though the auctioneer has not bid.
  Cosmetic; the Go port rendered the same thing (`modern_art.go:230-238`).
- recommendation: Only render the current-bid line when `bid > 0`, or have `pub_state`
  return `None` until a real bid exists.

### Sealed/once-around bid ties are broken in favor of the auctioneer
- severity: nit
- category: correctness
- location: game/modern-art-2/src/lib.rs:193
- finding: `highest_bidder` iterates turn order starting at `current_player` (the
  auctioneer) with a strictly-greater comparison, so on tied bids the auctioneer wins, then
  the player closest clockwise. Common editions break ties starting from the player to the
  auctioneer's *left* (auctioneer loses ties) — this reviewer could not confirm the exact
  edition rule, and the Go port is identical (`modern_art.go:496-506`), so flagging only as
  a nit for the Lead to adjudicate.
- recommendation: If official tie-break should exclude the auctioneer on ties, iterate from
  `current_player + 1` and handle the auctioneer last; otherwise document.

### `can_add` allocates a throwaway `Vec` via `unwrap_or(&vec![])`
- severity: nit
- category: quality
- location: game/modern-art-2/src/lib.rs:260
- finding: `!self.player_hands.get(player).unwrap_or(&vec![]).is_empty()` heap-allocates an
  empty Vec on every call purely as a fallback. `player` is always in range here, and even
  defensively `map_or` avoids the allocation.
- recommendation: `self.player_hands.get(player).is_some_and(|h| !h.is_empty())` (or
  `map_or(false, ...)` for older compilers).

### Guarded `bid.unwrap()` in the Open-auction arm of `whose_turn_players`
- severity: nit
- category: quality
- location: game/modern-art-2/src/lib.rs:152
- finding: `p != highest_bidder && (bid.is_none() || *bid.unwrap())` — the `unwrap()` is
  safe due to `||` short-circuiting, but repo rules forbid `.unwrap()` in runtime paths and
  `Option::is_none_or` (stable since 1.82, edition 2024 here) expresses it without the lint
  risk.
- recommendation: `bid.is_none_or(|b| b > 0)`.

### Redundant `use std::default::Default;` import
- severity: nit
- category: consistency
- location: game/modern-art-2/src/lib.rs:2
- finding: `Default` is in the standard prelude; the explicit import is dead weight.
- recommendation: Delete the line.

## Cross-references (not findings)

- `start_round` purchase-reset comment (lib.rs:291-293): in-code documented mirror of the
  Go quirk resetting `PlayerPurchases` inside the deal loop; net behavior identical.
- Once-around rule that the auctioneer may bid only if someone else has bid
  (`whose_turn_players`, lib.rs:165-181): identical in the Go port
  (`modern_art.go:470-482`) and documented as the behavior in RULES.md:71. Practically
  equivalent to official rules (an auctioneer bidding when nobody else did would only pay
  the bank unnecessarily).
- Double-auction "any player may add" (RULES.md:64-66) is serialized in the implementation:
  players in turn order from the auctioneer must `add` or `pass` (whose_turn Double arm,
  lib.rs:156-163), matching the Go port. Necessary for a turn-based digital adaptation.
- All rules deviations flagged above vs official Modern Art (all-cards payout, zero-count
  artists placing, settle_auction infinite loop, missing empty-hand skip on round
  transition, auctioneer-favorable tie-break) are inherited verbatim from the Go port —
  cross-unit decision needed on whether port parity or official rules win.
- The 4 binaries (`src/bin/modern_art_2_{cli,repl,fuzz,http}.rs`) are the standard
  boilerplate with no deviations (http binary's `expect` on ADDR parse is process startup).
  `Cargo.toml` is standard for the game crates; `tokio` "full" is only needed by the http
  binary — systemic pattern, tracked in the dependencies unit.

## Clean modules

- `src/card.rs` — clean. Deck composition (12/13/14/15/16 = 70 cards), suit/rank names,
  codes, and sort order all match the Go `cardDistribution` table and official card counts;
  `card_count` is exhaustive over the match so no missing-arm risk. Tests adequate.
- `src/command.rs` — clean. Parsers are gated by the same `can_*` predicates the state
  machine enforces; `bid_parser`'s Int bounds are kept valid by the `min_bid <= money`
  guard in `can_bid` (lib.rs:238, documented in-code); `price_parser` leaves max
  unbounded but `set_price` validates against money. No panics reachable from parse paths.
- `tests/contract.rs` — standard `assert_gamer_contract` harness, consistent with other
  crates.
- Test coverage in `src/lib.rs` is good (all five auction types, double-ends-round,
  round-end payouts, final placings, sealed-bid/money privacy in pub state). No test
  exercises the round-4 empty-hand or all-hands-empty scenarios flagged above — recommend
  adding regression tests when those are fixed.
