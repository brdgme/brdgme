# Verification: games-batch-f, for-sale-2 (F13-F23) — Worker W2

### F13 Hidden-info leak: selling-phase plays exposed via PubState.bids — CONFIRMED
- evidence: lib.rs:258 `self.bids[player] = building;` inside `play()`; lib.rs:412 `bids: self.bids.clone()` in `pub_state()` with no phase-conditional redaction; RULES.md:23 "Each player secretly selects one building to play"; render.rs:101-109 shows only `pub_state.bids[p]` for the viewer (`if let Some(p) = player`), so the UI hides it but the JSON PubState carries every already-played building during the Selling phase. Go parity confirmed: for_sale.go:54-63 `ToPubState` includes `Bids: g.Bids` verbatim, and for_sale.go:328 stores plays into `g.Bids` identically.
- severity: agree (major). Genuine hidden-information violation in the API contract for a simultaneous-secret-selection phase; any bot/API consumer gains a real strategic edge. Not critical (no data loss/crash), squarely major correctness.
- recommendation-check: valid. Zeroing `bids` in `pub_state()` while `current_phase() == Phase::Selling` and having `player_state()` re-insert the viewer's own entry into `public.bids[player]` is behavior-preserving: the Selling renderer only reads `pub_state.bids[p]` for the viewer (render.rs:101-109), spectator PubState render never reads bids in Selling, `whose_turn`/resolution use `finished_bidding` and internal state, not PubState. Buying-phase bids stay public as they should. The alternative (separate private field) is also sound.

### F14 Passing pays floor(bid/2); official rules round up — CONFIRMED (external basis for the official-rules claim)
- evidence: lib.rs:233 `let half_bid = self.bids[player] / 2;` (i32 division floors for non-negative bids); Go for_sale.go:296 `halfBid := g.Bids[player] / 2` identical; RULES.md:17 "paying half your current bid (rounded down)" documents the implemented behaviour. The "official rules round up" claim rests on external rulebook knowledge — noted as external basis, consistent with common For Sale editions.
- severity: agree (minor). Documented, Go-parity-faithful deviation; a rules-fidelity choice, not a defect in the port.
- recommendation-check: valid. `.div_ceil(2)` on a non-negative i32 bid gives round-up correctly; conditional on wanting fidelity, with RULES.md update. Leaving as-is also explicitly sanctioned.

### F15 Deck/chip setup deviates from official For Sale — CONFIRMED (external basis for official numbers)
- evidence: lib.rs:19 `pub const STARTING_CHIPS: i32 = 15;` flat; lib.rs:85-87 `building_deck()` = `(1..=20)`; lib.rs:89-91 `cheque_deck()` = `(1..=20).map(|i| if i < 3 { 0 } else { i })` -> {0,0,3..=20}; lib.rs:375-381 removes 2 per deck only when `players == 3`. Go identical: for_sale.go:421-429 (buildings 1..=20), :431-439 (i<3 -> 0), :92 `g.Chips[p] = 15`, :96-102 (3p removes 2). Official 30/30/scaled-chips figures are external basis.
- severity: agree (minor). Deliberate Go-compatible variant, documented in RULES.md; cross-reference only.
- recommendation-check: valid (explicitly "none required for parity").

### F16 RULES.md cheque deck description factually wrong — CONFIRMED
- evidence: RULES.md:8 "30 cheques: two 0s, then 2..=20." vs code (lib.rs:89-91): 20 cheques, values {0, 0, 3..=20} — count wrong, no 2s exist, and "two 0s, then 2..=20" would total 21 cards anyway. RULES.md:31 "Ties share a place." contradicts `placings()` (lib.rs:332-337) which passes `[player_points, chips]` metrics to `gen_placings` — equal totals are broken by remaining chips, so only players tied on both share a place. (RULES.md:7's building line at least carries the "(20 are used here.)" caveat; the cheque line has none.)
- severity: agree (minor quality). Player-facing documentation error shipped via `rules()` (lib.rs:517-519).
- recommendation-check: valid. "20 cheques: two 0s, then 3..=20" matches the code exactly; amending the tie sentence to mention the chips tie-break matches `placings()`.

### F17 End-of-game "scores" log shows cheque totals only — CONFIRMED
- evidence: lib.rs:110-127 Finished branch of `start_round()` labels the table "The game has finished!  The scores are:" but renders only `Self::deck_value(&self.cheques[p])` (lib.rs:118), omitting chips; final score is `player_points` = cheques + chips (lib.rs:328-330). `command()` then appends `placings_log(&self.placings(), Some(&scores))` built from `player_points` (lib.rs:449-453, 466-471, 484-489) — two differing "score" tables on game end. Go identical: for_sale.go:151-165 renders `g.DeckValue(g.Cheques[pNum])` only.
- severity: agree (minor). Misleading log, no state/scoring impact; the authoritative placings log is correct.
- recommendation-check: valid. Rendering `player_points(p)` (or cheques + chips columns) in the finished table is a trivial, safe change.

### F18 Phase inferred from deck sizes via SELL_THRESHOLD magic — CONFIRMED; recommended fix INVALID as stated
- evidence: lib.rs:20 `const SELL_THRESHOLD: usize = 18;`; lib.rs:94-104 `current_phase()` returns Buying while `!building_deck.is_empty() || (!open_cards.is_empty() && cheque_deck.len() >= SELL_THRESHOLD)`. Works only because the smallest pre-first-selling-draw cheque deck is 18 (3p) and every first selling draw drops it to 15/16/15 for 3/4/5p — any deck-size or player-count change silently breaks the discriminator. `Phase` enum (lib.rs:22-29) is never stored on `Game` (struct lib.rs:31-47 has no phase field; PubState.phase is computed at lib.rs:406). `status()` (lib.rs:386-401) independently re-derives finished from `open_cards/building_deck/cheque_deck` all empty — logically equivalent to `current_phase() == Finished` today, but a second source of truth. Go has the same magic 18 (for_sale.go:107-115).
- severity: agree (minor quality). Correct today, fragile under change.
- recommendation-check: INVALID as stated. A plain serde-defaulted `phase: Phase` field defaults to `Phase::Buying` (`#[derive(Default)]` with `#[default] Buying`, lib.rs:24-26); deserialising an in-flight game currently in the Selling phase would resurrect it as Buying, breaking `can_play`/`whose_turn` for live games. Serde default fns cannot see sibling fields, so the default cannot be computed from the decks. A sound migration needs e.g. `Option<Phase>` (or a sentinel) with `current_phase()` used as fallback when absent, or a post-deserialize fixup hook. The rest of the recommendation (explicit transitions in `start_round`, `status()` delegating to `current_phase()`) is sound.

### F19 Panic-on-empty-deck paths from corrupt state — CONFIRMED
- evidence: lib.rs:133 and :144 `split_off(self.*_deck.len() - n)` — usize underflow panic when deck len < players; lib.rs:266 and :282 `self.open_cards.remove(0)` — panic on empty; lib.rs:153 `self.hands[p][0]` — the autoplay guard at :151 checks only player 0's hand (`hands.first().is_some_and(|h| h.len() == 1)`), so a corrupt state with an empty hand for p>0 panics. All unreachable through legal play: `start()` sizes decks as multiples of player count and draws are per-player.
- severity: agree (nit). Corrupt/hand-edited state only; would surface as a panic in the serving process rather than a GameError.
- recommendation-check: valid. `hands[p].first()` plus graceful short-deck handling is optional hardening with no behavioural change on legal states.

### F20 Selling autoplay keys off player 0's hand size — CONFIRMED
- evidence: lib.rs:151 `if self.hands.first().is_some_and(|h| h.len() == 1)`. All hands have equal size by construction (every buying auction gives each player exactly one building; every selling round consumes one per player). Go equivalent implicit invariant.
- severity: agree (nit). Implicit invariant, no reachable bug.
- recommendation-check: valid. `self.hands.iter().all(|h| h.len() == 1)` is equivalent on legal states (players >= 3 so the vec is non-empty) and strictly safer; also fixes the F19 :153 panic path for p>0 empty hands.

### F21 Tie ranking diverges from Go GenPlacings (dense vs standard competition) — CONFIRMED
- evidence: Go brdgme/placings.go:55-87 `GenPlacings` increments `curPlace++` once per unique metric group — dense ranking, two tied at top -> [1,1,2]. Rust lib/game/src/game.rs `gen_placings` does `cur_place += players.len()` — standard competition, [1,1,3]. Crate test lib.rs:792-807 (`test_placings_tie_standard_competition`) codifies [1,1,3]. Self-consistent within the crate; port-wide lib-level divergence.
- severity: agree (nit, consistency). Correctly scoped as a lib/game cross-reference.
- recommendation-check: valid ("track at the lib/game level; no per-crate action").

### F22 render::highest_bid duplicates game logic with different sentinel — CONFIRMED
- evidence: render.rs:40-50 re-implements the scan over `PubState` with `-1` accumulator but `if best > 0 { Some(...) } else { None }` as the no-bid test; lib.rs:316-326 game version returns the raw `(player, amount)` with `-1` meaning no live bid. Behaviourally consistent today because real bids are always >= 1 (`bid()` requires `amount > highest` and highest starts at 0 among cleared bids), but the two can drift.
- severity: agree (nit, simplicity). Small presentational duplication.
- recommendation-check: valid. Both options (accept, or a shared Option-returning helper) are reasonable; a shared helper would need to live on data both sides have (bids + finished_bidding), which PubState carries.

### F23 Helpers unnecessarily pub; player_state indexes unchecked — CONFIRMED
- evidence: lib.rs:131-345 — `start_buying_round`, `start_selling_round`, `clear_bids`, `deck_value`, `whose_turn_inner`, `take_first_open_card`, `next_bidder`, `highest_bid`, `points_int`, etc. are all `pub` though they are crate-internal plumbing (in-crate tests do not need `pub`). lib.rs:417-425 `player_state()` indexes `self.chips[player]`, `self.hands[player]`, `self.cheques[player]` unchecked — panics on out-of-range player, matching the pattern across game crates.
- severity: agree (nit, consistency).
- recommendation-check: valid. `pub(crate)`/private trims API surface with no functional change; bounds behaviour is a platform-wide convention, reasonably left alone.
