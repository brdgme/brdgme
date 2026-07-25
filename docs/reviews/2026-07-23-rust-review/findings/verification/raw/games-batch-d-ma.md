# Batch D verification: modern-art-2 (F34-F46)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust/game/modern-art-2 (commit f8763a5)
Go original: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go/modern_art_1/modern_art.go

## F34 settle_auction infinite loop when all hands empty

- verdict: CONFIRMED
- evidence: lib.rs:450-459:
  ```rust
  self.state = State::PlayCard;
  self.next_player();
  while self.player_hands[self.current_player].is_empty() {
      logs.push(Log::public(vec![
          N::text("Skipping "),
          ...
      self.next_player();
  }
  ```
  No all-hands-empty guard; each iteration also pushes a log, so before hanging it grows `logs` unboundedly.
  Reachability chain verified:
  - Round end trigger is ONLY the 5th card of an artist: lib.rs:423 `if self.suit_cards_on_table(c.suit) >= 5 { logs.extend(self.end_round()); }` inside add_card_to_auction. No other path calls end_round.
  - Round 4 (index 3) deals 0 cards: lib.rs:90-98 round_cards table `[10,6,6,0]/[9,4,4,0]/[8,3,3,0]`; start_round (lib.rs:294-301) only deals when num_cards > 0 and never clears hands, so hands persist across rounds.
  - Purchases (which feed suit_cards_on_table, lib.rs:113-124) are reset every round start (lib.rs:293), so per-round counts cap at 4 per artist without triggering.
  - Therefore: reach round 4 with a small number of leftover cards (e.g. 3 cards of 3 different artists), play them all without any artist hitting 5 this round; the final card's auction settles via settle_auction with every hand empty -> infinite loop. Legal-play reachable. (Note: the loop can equally fire in rounds 2/3 if hands empty mid-round, since the skip loop runs before any round transition; round 1 is safe only because 3x10=30 > 20 max playable without a trigger... actually round 1 also cannot empty since max 4 per artist x 5 artists = 20 cards playable per round < cards in hand for 3p/4p; 5p: 8x5=40 > 20, same bound applies. Round >= 2 with carried-over small hands is the realistic path, round 4 the easiest.)
- Go parity: modern_art.go:689-695 `g.NextPlayer(); for len(g.PlayerHands[g.CurrentPlayer]) == 0 { ... g.NextPlayer() }` - identical (finding cited :690, correct).
- severity: upheld critical - unkillable busy loop in a server game runtime, reachable via legal play.
- evidence basis: code only.

## F35 end_round round-4 transition soft-lock on empty next hand

- verdict: CONFIRMED
- evidence: lib.rs:367-370:
  ```rust
  self.round += 1;
  self.next_player();
  logs.extend(self.start_round());
  ```
  No empty-hand skip here; the skip loop exists only in settle_auction (lib.rs:452). Transitions 1->2 and 2->3 deal cards so hands are refilled; transition 3->4 deals 0 (round_cards, lib.rs:90-98). If the player after the round-3-ending auctioneer has an empty hand:
  - whose_turn_players (lib.rs:144-145): `State::PlayCard => vec![self.current_player]` - returns only them.
  - command_parser (command.rs:17-42): can_play is true (lib.rs:208-210 checks turn + state only, not hand contents), so a `play` parser is offered, but cards_parser (command.rs:44-48) is `Enum::exact` over an empty hand - nothing parses. can_pass is false outside auctions (lib.rs:212-227). All other can_* require is_auction. No other skip mechanism exists anywhere (grep: next_player called only at lib.rs:369 and settle_auction).
  - status() (lib.rs:605-617) reports Active with whose_turn = [the stuck player]. Soft-lock.
- Go parity: modern_art.go:432-434 `g.Round += 1; g.NextPlayer(); g.StartRound()` - identical (cited lines correct).
- severity: upheld major.
- evidence basis: code only.

## F36 end-of-round payout pays cumulative value for non-top-3 artists

- verdict: ADJUSTED (facts confirmed; severity major -> minor)
- evidence: lib.rs:336-349:
  ```rust
  for p in 0..self.players {
      let mut p_total = 0;
      for c in &self.player_purchases[p] {
          p_total += self.suit_value(c.suit);
      }
      ...
      self.player_money[p] += p_total;
  }
  ```
  suit_value (lib.rs:126-131) sums the artist's value across ALL rounds on the value board, and the loop pays it for every purchased card regardless of whether the artist placed top-3 this round. Official Modern Art: only paintings by this season's top-3 artists are sold to the bank (at cumulative value); others pay nothing. So an artist ranked in round 1 but unranked in round 2 still pays its round-1 value for round-2 purchases here - a real deviation.
  Go parity: modern_art.go:405-415 identical (cited 406-415, correct).
  RULES.md check: RULES.md:102-105 explicitly documents the implemented behavior: "Every player is then paid, for **each** card they've purchased so far this round, the *total* cumulative value of that card's artist - even if the artist didn't place this round." (Finding cited :103-104; the sentence spans 102-105.)
- severity: corrected to minor. The behavior is a faithful port of the shipped Go game and is explicitly documented in the crate's own rules text, which is what players are told. It is a deviation from official Knizia rules, so it remains worth flagging (the doc may simply canonize an inherited defect), but "judged against official rules = major" does not hold when the crate deliberately documents and ships different rules (batch-c precedent).
- evidence basis: code + RULES.md + own knowledge of official Modern Art scoring (high confidence: only top-3 artists' paintings are sold each season).

## F37 zero-card artists ranked and awarded $20/$10

- verdict: CONFIRMED
- evidence: lib.rs:310-326: counts is seeded for ALL suits, including zero-count ones:
  ```rust
  for s in suits() {
      counts.insert(s, self.suit_cards_on_table(s));
  }
  ...
  for &v in &[30, 20, 10] {
      let mut highest = suits()[0];
      let mut highest_count: i64 = -1;
      for s in suits() {
          if !*scored.get(&s).unwrap_or(&false) && counts[&s] as i64 > highest_count {
  ```
  Because every suit is in counts (0 included) and the sentinel is -1, a 0-count artist beats the sentinel and gets ranked/valued once the genuinely-played artists are exhausted. Scenario is reachable: end_round clears currently_auctioning first (lib.rs:309), so a round ended by 5 straight cards of one artist (e.g. via a fast Double) can leave only 1 artist with count > 0 (4 purchased; the trigger card excluded), yet 3 artists get $30/$20/$10 - two of them with 0 cards. That inflated value then also feeds the F36 payout and future rounds' cumulative values.
  Go parity: modern_art.go:385-403 - counts seeded for all suits (385-388), `highestCount := -1` (391), same strictly-greater scan - identical (cited 389-403, correct).
  RULES.md:86-96 says "the three artists with the most cards played this round ... are ranked and awarded" with "Any remaining artists: $0 this round" - it implies three artists are always awarded but never explicitly states 0-card artists earn value, so unlike F36 this is not clearly documented behavior.
- severity: upheld major.
- evidence basis: code + RULES.md + own knowledge (moderate-high confidence: official rules only rank artists with at least one painting sold; if fewer than three artists sold, the remaining values are not assigned).

## F38 round_cards unreachable!() and unchecked index

- verdict: CONFIRMED
- evidence: lib.rs:90-98:
  ```rust
  fn round_cards(players: usize, round: usize) -> usize {
      let table: [usize; 4] = match players {
          3 => [10, 6, 6, 0],
          4 => [9, 4, 4, 0],
          5 => [8, 3, 3, 0],
          _ => unreachable!(),
      };
      table[round]
  }
  ```
  `unreachable!()` panics for players outside 3..=5, `table[round]` panics for round > 3. Guards verified: Game::start validates player count (lib.rs:582-588) and end_round sets finished at ROUNDS-1 (lib.rs:350,366) instead of incrementing, so round stays <= 3 in normal flow. But Game derives Deserialize (lib.rs:34) with pub fields, so a hostile/corrupt persisted state reaches the panic. Repo convention forbids panic macros in runtime paths.
- severity: upheld minor/quality.
- evidence basis: code only.

## F39 RULES.md says winner goes next; code passes clockwise

- verdict: CONFIRMED
- evidence: RULES.md:73-76: "Whoever wins the auction pays their bid ... The winner adds the card(s) to their purchases (face-up, public knowledge) and it becomes their turn next." (finding cited :76 - the clause is on line 76.) Implementation: settle_auction lib.rs:450-451 `self.state = State::PlayCard; self.next_player();` - next turn goes to the player after current_player (the auctioneer, or the adder in a Double per lib.rs:414), never to the winner. Tests confirm (e.g. lib.rs:846-847: STEVE wins, ELVA - after auctioneer BJ - is current). Code matches official rules (play passes left of the auctioneer) and Go (modern_art.go:688-689); the doc is what is wrong.
- severity: upheld minor/consistency (doc fix).
- evidence basis: code + RULES.md + own knowledge (high confidence official: next auctioneer is left of previous).

## F40 RULES.md Double section omits Once Around as second card

- verdict: CONFIRMED
- evidence: RULES.md:63-64: "**Double** - Works like Open, Fixed Price, or Sealed depending on the second card added". add_card (lib.rs:385-402) rejects only mismatched artist (391) and `c.rank == Rank::Double` (396-399); a Once Around card is accepted, and whose_turn_players handles Some(Rank::OnceAround) as an auction type (lib.rs:165-181). Doc list is incomplete.
- severity: upheld minor/consistency (doc fix).
- evidence basis: code + RULES.md.

## F41 game-end leaves State::Auction; final render shows empty auction

- verdict: CONFIRMED
- evidence: trace of the 5th-card game-end path: play_card -> add_card_to_auction sets `self.state = State::Auction` (lib.rs:422) and pushes the card, then lib.rs:423 fires end_round. On the final round end_round sets `self.finished = true` (lib.rs:366) without touching state (non-final rounds get State::PlayCard via start_round, lib.rs:279). It does clear currently_auctioning (lib.rs:309) and the 5th card is indeed never sold - no auction runs. Result: finished=true, state=Auction, auctioning empty. pub_state (lib.rs:624) reports is_auction=true; render.rs:57-61 then prints "<player> is auctioning " with card_names of an empty vec. Worse than stated: since auction_type() is None (empty currently_auctioning) != Some(Sealed), pub_state.current_bid = Some(highest_bidder()) (lib.rs:628-632) with bids reset (lib.rs:421) yields (auctioneer, 0), so render.rs:62-71 also prints "Current bid: $0 by <auctioneer>" on the final screen. whose_turn_players and status do short-circuit on finished (lib.rs:141-143, 605-610), so this is display-only.
- severity: upheld minor/correctness (cosmetic final-screen garbage).
- evidence basis: code only.

## F42 "Current bid: $0 by auctioneer" before any bid

- verdict: CONFIRMED
- evidence: lib.rs:628-632: `current_bid: if self.is_auction() && self.auction_type() != Some(Rank::Sealed) { Some(self.highest_bidder()) } else { None }`. highest_bidder (lib.rs:190-202) starts bid=-1 and iterates from current_player, so with no bids the first probe (auctioneer, 0) wins: returns (auctioneer, 0). render.rs:62-70 prints "Current bid: $0 by <auctioneer>". Go parity: modern_art.go:230-237 renders identically from HighestBidder with no bid guard.
- severity: upheld nit/consistency.
- evidence basis: code only.

## F43 auctioneer wins ties in sealed/once-around

- verdict: CONFIRMED
- evidence: lib.rs:190-202: iteration `for i in self.current_player..self.current_player + self.players` starts at the auctioneer; comparison `if b > bid` is strictly greater, so the first-seen (auctioneer, then clockwise) keeps ties. Go parity: modern_art.go:496-506 identical (`for i := g.CurrentPlayer; ...`, `if g.Bids[p] > bid`). Cited lines correct. Note: for sealed auctions this actually matches the common official tie-break (auctioneer wins ties, else nearest clockwise), so nit is the right framing.
- severity: upheld nit.
- evidence basis: code + own knowledge (moderate confidence on official sealed tie-break).

## F44 can_add allocates throwaway Vec

- verdict: CONFIRMED
- evidence: lib.rs:260: `&& !self.player_hands.get(player).unwrap_or(&vec![]).is_empty()` - allocates a fresh Vec on every call solely as an unwrap_or default. `.map_or(false, ...)` / `is_some_and` would avoid it.
- severity: upheld nit/quality.
- evidence basis: code only.

## F45 .unwrap() in whose_turn_players Open branch

- verdict: ADJUSTED (claim right; quoted expression wrong)
- evidence: lib.rs:150-153 actual code:
  ```rust
  let bid = self.bids.get(&p);
  p != highest_bidder && (bid.is_none() || *bid.unwrap() > 0)
  ```
  The finding quoted `(bid.is_none() || *bid.unwrap())`, dropping the `> 0` - as quoted it would not even compile (i32 where bool expected). The substantive claim stands: `.unwrap()` in a runtime path, safe only via the `is_none()` short-circuit, against repo convention; `bid.map_or(true, |&b| b > 0)` expresses it without unwrap.
- severity: upheld nit/quality.
- evidence basis: code only.

## F46 redundant `use std::default::Default;`

- verdict: CONFIRMED
- evidence: lib.rs:2 `use std::default::Default;`. Cargo.toml:6 `edition = "2024"`. Default is in the prelude of every edition; the import is redundant.
- severity: upheld nit/consistency.
- evidence basis: code + Cargo.toml.

## Cross-cutting observations

- F34 and F35 are the same missing invariant ("current player must have a card or be skippable") surfacing at two sites; a shared fix (skip-with-termination-check, ending the round/game when no hands remain) addresses both.
- F36 and F37 compound: F37's phantom artist values inflate the cumulative board that F36 then pays out on every card.
- Every behavioral finding checked is a faithful Go port (settle loop 689-695, end-round transition 432-434, payout 405-415, ranking 385-403, highest-bidder 496-506, current-bid render 230-237); none are porting regressions.
- Go line numbers cited by the original review were accurate throughout (off by at most 1-4 lines where ranges were cited).
