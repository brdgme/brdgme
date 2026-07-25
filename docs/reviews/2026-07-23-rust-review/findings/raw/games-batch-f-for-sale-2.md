# for-sale-2 review findings (2026-07-23)

Crate: `rust/game/for-sale-2/` (~1,108 LOC). Port of `brdgme-go/for_sale_1/`.
Files reviewed: `src/lib.rs` (846 lines incl. unit tests), `src/command.rs`,
`src/render.rs`, `tests/contract.rs`, `Cargo.toml`, `RULES.md`. The 4 `src/bin/`
binaries were skipped per instructions (boilerplate, tracked systemically).

Overall: the port is very faithful to the Go original — structure, log texts,
and quirks all match. Core auction/selling logic is correct for the ported
ruleset, turn handling is sound, and no reachable panic path from normal
player input was found (bid amounts are bounded by both parser `max` and
game-side checks; negative bids are rejected by the "must bid higher" check;
the `next_bidder` loop is only entered with >=2 non-finished players).
Test coverage is good (13 unit tests + the shared gamer-contract test).

### Hidden-info leak: selling-phase plays exposed via PubState.bids
- severity: major
- category: correctness
- location: game/for-sale-2/src/lib.rs:258 (play stores into `bids`), surfaced at game/for-sale-2/src/lib.rs:411-412 (`pub_state` clones `bids`/`finished_bidding` verbatim)
- finding: The selling phase is supposed to be simultaneous secret selection ("Each player secretly selects one building to play", RULES.md:23). `play()` records the played building in `self.bids[player]`, and `pub_state()` exposes the full `bids` vector to every player. Any client (bot or API consumer) reading `pub_state` during the selling phase can see exactly which building each already-played opponent chose before picking their own. The HTML renderer (render.rs:101-109) only shows the viewer's own play, so the leak is invisible in the UI but present in the JSON contract. Cross-reference: the Go original has the identical leak (`ToPubState` includes the `Bids` map, for_sale.go:54-64) — this is a faithfully preserved Go flaw, not a new divergence, but it is a genuine hidden-information violation.
- recommendation: Redact other players' `bids` entries while `phase == Selling` in `pub_state()` (e.g. zero out bids of players other than... nothing — pub_state is shared, so zero all of them and let each `player_state` re-add its own), or store played selling cards in a separate private field instead of reusing `bids`.

### Passing pays floor(bid/2); official rules round the payment up
- severity: minor
- category: correctness
- location: game/for-sale-2/src/lib.rs:233
- finding: `let half_bid = self.bids[player] / 2;` floors, so passing on a bid of 5 costs 2. Official For Sale rules: a passing player pays half their bid **rounded up** (keeps half rounded down), i.e. 5 -> pays 3. Cross-reference: Go does the same integer division (`halfBid := g.Bids[player] / 2`, for_sale.go:296), so this is a preserved Go quirk, and RULES.md:17 honestly documents the implemented ("rounded down") behaviour. Noted only as a rules-fidelity cross-reference.
- recommendation: If rules fidelity is desired: `let half_bid = self.bids[player].div_ceil(2);` plus RULES.md update. Otherwise leave as-is and keep the cross-reference note.

### Deck/chip setup deviates from official For Sale (preserved Go quirk)
- severity: minor
- category: correctness
- location: game/for-sale-2/src/lib.rs:19 (STARTING_CHIPS=15 for all counts), 85-91 (20 buildings 1..=20; 20 cheques {0,0,3..=20}), 375-381 (3p removes 2 cards per deck)
- finding: Official: 30 property cards (1-30), 30 cheques (two each of 0 and 2..=15), 3p removes 6 cards per deck, 4p removes 2 per deck, and starting chips vary by player count. This port (like Go) uses 20 buildings, 20 cheques (two 0s then 3..=20 — no 2s, values to 20), removes 2 per deck only for 3p, removes nothing for 4p, and gives a flat 15 chips. Cross-reference only — Go behaves identically (for_sale.go:92, 96-102, 421-443) and RULES.md documents the ported variant.
- recommendation: None required for the port; keep documented as a deliberate Go-compatible variant.

### RULES.md cheque deck description is factually wrong
- severity: minor
- category: quality
- location: game/for-sale-2/RULES.md:7-8
- finding: "30 cheques: two 0s, then 2..=20" — the code builds **20** cheques with values {0, 0, 3, 4, ..., 20} (lib.rs:89-91): there is no 2, and there are 20 cards, not 30. Line 7's "(20 are used here)" for buildings is fine, but the cheque line matches neither the code nor official rules. Also RULES.md:31 says "Ties share a place", but `placings()` tie-breaks equal totals on remaining chips (lib.rs:332-337), so ties on points are actually broken unless chips are also equal.
- recommendation: Fix to e.g. "20 cheques: two 0s, then 3..=20" and amend the tie sentence to mention the chips tie-break (which matches official rules).

### End-of-game "scores" log shows cheque totals only, not final scores
- severity: minor
- category: correctness
- location: game/for-sale-2/src/lib.rs:118 (only `deck_value(&self.cheques[p])` rendered), 122-126 ("The game has finished!  The scores are:")
- finding: The Finished-branch log labels the table "The scores are" but shows only each player's cheque sum, omitting leftover chips which are part of the final score (`player_points`, lib.rs:328-330). Because the game normally ends via the selling autoplay (`start_selling_round` -> `play` -> `start_round`), this misleading table is always emitted; `command()` then appends a correct `placings_log` with real scores, so players see two contradictory "score" summaries. Cross-reference: Go is identical (for_sale.go:144-173).
- recommendation: Render `player_points(p)` (or cheques + chips as separate columns) in the finished table.

### Phase inferred from deck sizes via SELL_THRESHOLD magic constant
- severity: minor
- category: quality
- location: game/for-sale-2/src/lib.rs:20 (`SELL_THRESHOLD: usize = 18`), 94-104 (`current_phase`)
- finding: Whether `open_cards` holds buildings or cheques is inferred by `cheque_deck.len() >= 18` — a magic number that happens to equal the 3-player starting cheque count and works only because the first selling draw always drops the deck below 18 (18->15, 20->16, 20->15). The `Phase` enum exists (lib.rs:22-29) but is never stored; the state could carry a `phase` field instead. The current trick is correct today but silently breaks if deck sizes, player counts, or the 3p removal ever change. Additionally `status()` (lib.rs:386-401) re-implements the finished predicate from raw deck emptiness instead of reusing `current_phase()`, giving two sources of truth for the same concept.
- recommendation: Store `phase: Phase` in `Game` (serde-defaulted for migration) and transition it explicitly in `start_round`; have `status()` delegate to `current_phase()`.

### Panic-on-empty-deck paths reachable only from corrupt/deserialised state
- severity: nit
- category: correctness
- location: game/for-sale-2/src/lib.rs:133 and 144 (`split_off(len - n)` panics if deck shorter than player count), 266 and 282 (`open_cards.remove(0)` panics on empty), 153 (`hands[p][0]` if a hand is unexpectedly empty during autoplay)
- finding: All are unreachable through legal play (deck sizes are multiples of the player count by construction, an auction always has >=1 open card while someone can act, and the `next_bidder` loop at 307-312 is only entered with >=2 non-finished players so it cannot spin). But `Game` is fully `pub` + `Deserialize`, so a corrupted/migrated state blob would panic the HTTP request rather than return an error. Normal crafted *input* (negative/huge bids, wrong cards, out-of-turn commands) is handled cleanly.
- recommendation: Optional hardening: guard the autoplay with a per-player `hands[p].first()` and let `start_*_round` no-op/error on short decks. Low priority.

### Selling autoplay keys off only player 0's hand size
- severity: nit
- category: quality
- location: game/for-sale-2/src/lib.rs:151
- finding: `self.hands.first().is_some_and(|h| h.len() == 1)` assumes all hands have equal size. True by construction, and identical to Go (`if g.Hands[0].Len() == 1`, for_sale.go:194), so cross-reference nit only.
- recommendation: None needed; if touched, use `self.hands.iter().all(|h| h.len() == 1)`.

### Tie ranking diverges from Go GenPlacings (dense -> standard competition)
- severity: nit
- category: consistency
- location: game/for-sale-2/src/lib.rs:332-337 and test at 792-807
- finding: Go `GenPlacings` increments `curPlace` by 1 per group (dense ranking: two tied at top -> [1,1,2]); Rust `gen_placings` (lib/game/src/game.rs:154-179) adds the group size (standard competition: [1,1,3]), and this crate's test `test_placings_tie_standard_competition` codifies the Rust behaviour. This is a lib-level port-wide divergence, not specific to for-sale-2 — cross-reference for the Lead to track at the lib/game level. Within this crate the behaviour is self-consistent and tested.
- recommendation: Track as a lib/game port-divergence item; no per-crate action.

### Known consumer: doc_int min:None help-rendering bug
- severity: nit
- category: correctness
- location: game/for-sale-2/src/command.rs:43-46
- finding: `bid_parser` uses `Int { min: None, max: Some(max) }` — a known consumer of the lib/game `doc_int` bug where help text renders wrong for unbounded-min ints. Cross-reference only per instructions; game logic itself rejects out-of-range and negative bids correctly (lib.rs:203-215).
- recommendation: None at crate level; fixed when the lib bug is fixed.

### Known consumer: binary-only deps declared as library dependencies
- severity: nit
- category: dependencies
- location: game/for-sale-2/Cargo.toml:9-16
- finding: `brdgme_cmd`, `brdgme_fuzz`, and `tokio` (features = ["full"]) are library `[dependencies]` but are only used by the `src/bin/` targets. Systemic across the 27 game crates — cross-reference only.
- recommendation: None at crate level.

### render::highest_bid duplicates game logic with a different sentinel
- severity: nit
- category: simplicity
- location: game/for-sale-2/src/render.rs:40-50 vs game/for-sale-2/src/lib.rs:316-326
- finding: The renderer re-implements `highest_bid` over `PubState` with `best > 0` as the "no bid" test (game code uses -1 and guarantees bids are >= 1, so both work). Duplicated logic can drift; the renderer version also silently treats a (currently impossible) bid of 0 as "none".
- recommendation: Either accept the duplication (it's small and presentational) or expose an Option-returning helper on the game and reuse it.

### Helper methods unnecessarily pub; player_state indexes unchecked
- severity: nit
- category: consistency
- location: game/for-sale-2/src/lib.rs:131-345 (all helpers `pub`), 417-425 (`self.hands[player]` etc. index without bounds check)
- finding: `clear_bids`, `take_first_open_card`, `next_bidder`, `highest_bid`, `deck_value`, `start_*_round` are crate-internal plumbing exposed as `pub`. `player_state()` panics on an out-of-range player index (framework presumably validates; same pattern across game crates). Neither is exploitable through normal flows.
- recommendation: Trim visibility to `pub(crate)`/private where not used by bins; leave indexing as-is if other crates share the pattern.

## Clean aspects (verified)
- Two-phase auction/selling resolution logic is correct: pass takes lowest open card and pays floored half; last remaining bidder takes the highest for full bid and leads the next round; selling resolves lowest-building -> lowest-cheque with unique cards so no tie ambiguity; final selling round autoplays correctly.
- Player-count validation (3-5) and per-count round arithmetic (18/3, 20/4, 20/5) are consistent.
- No panic reachable from legal or adversarial *command input*; parser + game-side double checks on bid bounds.
- Undo semantics sensible (bid undoable, pass/play not).
- `points()` gated on finished, matching Go.
- Tests are thorough: full-game simulation, error paths, pass/last-bidder economics, selling resolution, finished-state commands, placings (incl. tie), pub/player state redaction of hands/cheques, plus the shared `assert_gamer_contract` test.
