# Verification: games-batch-d (unit 6)

Independent verification of `findings/games-batch-d.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Crates: `game/lords-of-vegas-1`, `game/jaipur-2`, `game/sushi-go-2`,
`game/modern-art-2`. Raw verdict dumps: `raw/games-batch-d-lov.md`,
`raw/games-batch-d-jaipur.md`, `raw/games-batch-d-sushigo.md`,
`raw/games-batch-d-ma.md`. Process log: `games-batch-d-LOG.md`.

Go sources: modern_art_1 and sushi_go_1 exist in the snapshot's brdgme-go
and were used for every port-parity claim; lords_of_vegas and jaipur have
no Go source, so their official-rules claims were checked for internal
consistency and their evidence basis flagged.

## Per-finding verdicts

### lords-of-vegas-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | unimplemented!() in command dispatch | major | CONFIRMED | Five unimplemented!() arms at lib.rs:182-186; only build/done wired (command.rs:20-27); the five other parsers are complete and pub (command.rs:49-152), one line from making a valid command panic |
| F2 | HashMap/HashSet iteration feeds boss-tie RNG | major | CONFIRMED | HashSet pop (board.rs:248) and TILES HashMap keys (board.rs:278) order which tile consumes which reroll AND (via the recursive re-tie pass, board.rs:337) how many draws occur; seeded-replay divergence real; Loc derives Ord (board.rs:73) so the fix is feasible |
| F3 | resolve_boss_ties never populates logs | minor | CONFIRMED | logs only extended from the always-empty recursive result; reroll value discarded at board.rs:331; build() extends empty vec and sets can_undo=false (lib.rs:308-311) — silent rerolls, RULES.md:85-88 describes them as player-facing |
| F4 | usize underflow in renderer supply math | minor | ADJUSTED (minor -> major) | Dice/token halves latent as stated, but the CASINO_TILES half (render.rs:117) is reachable in ordinary 5-6p play: build() (lib.rs:251-314) imposes no supply or colour limit, so 10+ same-colour builds exceed CASINO_TILES=9 and every render panics (debug) / wraps (release) |
| F5 | Loc::parse_str unvalidated; lot 0 underflows neighbours | minor | CONFIRMED | parse_str (board.rs:80-91) checks nothing against max_lot; `self.lot - 1` underflows for lot 0 (board.rs:107); player path safe via Enum::exact |
| F6 | lazy_static vs OnceLock | minor | CONFIRMED | Sole use tile.rs:3,23-25; edition 2024 has LazyLock for free |
| F7 | unreachable!() in starting-cash fold | nit | ADJUSTED (details; nit stands) | Unreachability proof holds but min insert position is 38 (2p), not 39, and card.rs:27-29 DOES carry the invariant comment — the missing comment is at lib.rs:118 itself |
| F8 | serde_json runtime dep, test-only use | nit | CONFIRMED | Cargo.toml:18; sole use lib.rs:350 in #[cfg(test)] |
| F9 | Redundant FromIterator import | nit | CONFIRMED | board.rs:3; crate is edition 2024 (Cargo.toml:6), prelude covers it |
| F10 | Hardcoded 3 vs BLOCK_WIDTH | nit | CONFIRMED | render.rs:154-155 vs board.rs:16 (BLOCK_WIDTH module-private, trivial visibility bump) |
| F11 | Casino colours vs RULES.md | nit | CONFIRMED | RULES.md:48-54 Tan/olive + Brick red vs casino.rs:27-35 Orange + Brown |
| F12 | Player counts 2-6 vs official 2-4 | nit | CONFIRMED (external basis) | Code allows 2-6 (lib.rs:97-103, 225-227); RULES.md silent on player count; the 2-4 claim rests solely on external rules knowledge (no Go source) |

### jaipur-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F13 | 8 camels vs official 11 / 52 vs 55 cards | major | REJECTED | card_count is the post-setup composition: start_round conjures the 3 market camels out of thin air (lib.rs:223) rather than drawing them, so 8 deck camels + 3 market camels = 11 in play and the deck ends at 40 exactly as official (55 - 3 - 10 dealt - 2 replenish). The recommended fix (camels=11) would create 14 camels — a bug. Lead independently re-traced setup and concurs |
| F14 | No bonus token for 6/7-card sales | major | CONFIRMED | bonuses.get_mut(&quantity) with keys exactly 3/4/5 (lib.rs:521); HAND_SIZE 7 makes 6/7-sales legal; contradicted in-repo by render.rs:153 ("5 or more") and DATA_DOCS.md:13 ("3+") — genuine scoring defect |
| F15 | Next-round starter is not the round loser | major | ADJUSTED (major -> minor) | All code facts confirmed: no path sets the starter; sell-triggered round end skips next_player() (lib.rs:571-575) so the seller starts; deck-exhaustion paths (lib.rs:343-345, 474-476) likewise. But the "official rule: loser starts" premise is external-only and uncorroborated in-repo (RULES.md is a stub). Restore major if the rulebook quote is confirmed at adjudication |
| F16 | Camel token counted as bonus token in tie-break | minor | CONFIRMED (external basis) | bonus_tokens[cw] += 1 at lib.rs:598 feeds the first tie-break (lib.rs:617-620); camel-token/bonus-token distinction is external-rules based but consistent with verifier knowledge |
| F17 | RULES.md one-line stub | minor | CONFIRMED | File is exactly `# Jaipur`; rules() serves it (lib.rs:816-818); strategy/data docs exist with substance |
| F18 | Mixed-type sell silently coerced | minor | CONFIRMED | command.rs:76-85 keeps only goods[0]; sell() validates only the first good's count, so `sell dia gold lea` silently sells 3 diamonds if held |
| F19 | Dead is_empty branch in command_parser | nit | CONFIRMED | Two unconditional pushes precede the check (command.rs:16-23); real None at line 13 |
| F20 | unwrap_or(Good::Diamond) fallback | nit | CONFIRMED | Many::some_spaced has min=Some(1) (lib/game parser/mod.rs:310-317; errors below min at 382-391); fallback unreachable |
| F21 | Placings-log block duplicated | nit | CONFIRMED | Identical 11-line blocks at lib.rs:754-764 and 777-787 |
| F22 | "N rounds remaining" overstates | nit | CONFIRMED | render.rs:174 vs first-to-2 match end (lib.rs:648-650, DATA_DOCS.md:6) |
| F23 | Camel display hides count PubState exposes | nit | CONFIRMED | render.rs:40-42 "no"/"some" vs exact PubState.camels (test at lib.rs:1316) |

### sushi-go-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F24 | Round 2 passes right vs official always-left | minor | CONFIRMED | lib.rs:361-372 alternates by round parity; Go game.go:180-195 identical; RULES.md:9 documents it (self-consistent deviation, as the finding itself states) |
| F25 | Pudding all-tied 2p awards nothing; dummy dilutes | minor | CONFIRMED | first==last branch awards 0 (lib.rs:492) though 2p has no fewest penalty (lib.rs:513); dummy included via 0..all_players loop; Go game.go:288,306 identical |
| F26 | Placings pudding tiebreak vs "official rules leave ties" | minor | ADJUSTED (minor -> nit) | Code/test/Go/RULES.md facts all verify, but the core premise is wrong: the official Gamewright rulebook DOES break score ties by most puddings ("if there is a tie, the player with the most pudding cards wins"), so the implementation is correct. Residual: document the tiebreak in RULES.md (currently silent — Lead re-checked) |
| F27 | Vacuous test_hand_passing_left | minor | CONFIRMED | lib.rs:1398-1416: deliberate unwrap_err, direct end_hand(), self-deprecating comments, zero assertions; real coverage at lib.rs:1418-1438 |
| F28 | draw_count dead (2,9) entry + unwrap_or(9) | minor | CONFIRMED | Sole production call passes all_players (lib.rs:292); (2,9) exercised only by a test (lib.rs:1301); hand sizes 9/9/8/7 verified; Go deck.go:58 comments "Usually 10, but we implement the variant" |
| F29 | Pudding hint "least -6" false in 2p | nit | CONFIRMED | Static text lib.rs:123; penalty guarded at lib.rs:513 |
| F30 | Maki second-place <= 3 guard | nit | CONFIRMED | Guard wraps both award and log, but the only excluded case yields 3/4 == 0 points — score outcome identical, only the log suppressed, as claimed |
| F31 | render_name duplicated | nit | ADJUSTED (details; nit stands) | Logic duplication and the `player > players - 1` underflow expression confirmed in both copies, but not "byte-for-byte" (method vs free fn) |
| F32 | Dummy-slot guard order | nit | CONFIRMED | lib.rs:675-678 reads playing[DUMMY] before the players == 2 check; harmless today, panics only if all_players < 3 (impossible for started games) |
| F33 | command() placings-log duplication | nit | CONFIRMED | Verbatim blocks at lib.rs:839-856 and 857-874 |

### modern-art-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F34 | Infinite busy-loop when all hands empty after settle | critical | CONFIRMED | Skip loop lib.rs:452-459 has no all-hands-empty guard and pushes a log per iteration; only round-end trigger is the 5th-card check (lib.rs:423); round 4 deals 0 and hands persist, so playing out round 4 with no artist reaching 5 legally strands all hands empty; Go modern_art.go:689-695 identical |
| F35 | Round 4 starts on empty-handed player, soft-lock | major | CONFIRMED | end_round (lib.rs:367-370) has no skip; empty-handed starter gets whose_turn=[them] with a play parser over an empty Enum::exact (command.rs:44-48) and no pass outside auctions; no other skip mechanism exists; Go :432-434 identical |
| F36 | Payout pays all purchases incl. non-top-3 artists | major | ADJUSTED (major -> minor) | Code facts and Go parity (:405-415) confirmed, but RULES.md:102-105 explicitly documents the behavior ("even if the artist didn't place this round") — documented intended in-crate behavior per the batch-c precedent. Kept minor (not nit): material economy deviation from official rules; the doc may canonize an inherited defect — cross-unit adjudication pending |
| F37 | Zero-card artists ranked and awarded $20/$10 | major | CONFIRMED | Key trace the original skipped verified: counts map is seeded for ALL suits including zeros (lib.rs:310-313), so the -1 sentinel really ranks 0-card artists; reachable (5-0-0-0-0 round); NOT documented in RULES.md; Go :385-403 identical |
| F38 | unreachable!()/unchecked indexing in round_cards | minor | CONFIRMED | lib.rs:90-98; guarded in normal flow but Game is Deserialize with pub fields |
| F39 | RULES.md "winner takes next turn" wrong | minor | CONFIRMED | RULES.md:76 vs settle_auction next_player() from the seller (lib.rs:450-451); code matches official + Go; doc wrong |
| F40 | RULES.md Double omits Once Around | minor | CONFIRMED | RULES.md:63; add_card (lib.rs:396) only rejects a second Double |
| F41 | Game end leaves stale State::Auction | minor | CONFIRMED | Final 5th card sets State::Auction (lib.rs:422); end_round never resets; final screen shows "is auctioning" with empty list AND "Current bid: $0 by <auctioneer>" — slightly worse than stated; display-only (whose_turn/status short-circuit on finished) |
| F42 | "Current bid: $0" before any bid | nit | CONFIRMED | lib.rs:628-632 + highest_bidder -1 sentinel yields (auctioneer, 0); render.rs:62-70 prints it; Go :230-237 same |
| F43 | Sealed/once-around ties favor auctioneer | nit | CONFIRMED | Iteration from current_player, strictly-greater compare (lib.rs:190-202); Go :496-506 identical; nit framing right (matches a common official sealed tie-break) |
| F44 | can_add throwaway vec![] | nit | CONFIRMED | lib.rs:260 |
| F45 | Guarded bid.unwrap() | nit | ADJUSTED (details; nit stands) | Real expression is `(bid.is_none() || *bid.unwrap() > 0)` (lib.rs:152) — the original quote dropped `> 0` and would not compile; the unwrap-in-runtime-path substance stands |
| F46 | Redundant use std::default::Default | nit | CONFIRMED | lib.rs:2; edition 2024 |

## Summary

- Findings verified: 46
- CONFIRMED: 38, ADJUSTED: 7 (F4, F7, F15, F26, F31, F36, F45),
  REJECTED: 1 (F13), UNVERIFIABLE: 0
- Corrected tallies for the unit (45 surviving findings):
  1 critical / 6 major / 16 minor / 22 nit.
  Original per-finding fields: 1 critical / 8 major / 16 minor / 21 nit
  over 46. Note: the findings file's own summary line ("1 critical,
  9 major, 14 minor, 22 nit") miscounts its own blocks.
- Severity changes: F4 minor -> major (reachable render underflow);
  F15 major -> minor (external premise uncorroborated); F26 minor -> nit
  (premise refuted); F36 major -> minor (documented in-crate).
- Lead spot-checked every ADJUSTED/REJECTED verdict directly against the
  snapshot: F4 (lib.rs:251-314, lib.rs:29, render.rs:117), F7
  (card.rs:20-35 incl. the 2p insert-position math), F13 (lib.rs:97-107 +
  start_round lib.rs:213-244 full setup re-trace), F15 (accepted on W2's
  line-traces; premise flagged), F26 (RULES.md tiebreak silence re-checked;
  Lead concurs on the official pudding tiebreak, reclassified REJECTED ->
  ADJUSTED per batch-c precedent), F36 (RULES.md:96-105 read directly),
  F45 (dump-quoted expression).

## Notable corrections

- F13 (jaipur camel count) is the review's one outright miss: the original
  reviewer read the card_count table without tracing setup. start_round
  seeds the market with 3 camels conjured out of thin air (lib.rs:223), so
  the 8-camel/52-card deck is distributionally identical to the official
  55-card/11-camel game after setup (deck ends at 40 in both). Worse, the
  finding's recommended fix (Camel => 11) would have introduced a real bug
  (14 camels in play).
- F4 (lords-of-vegas render underflow) upgraded minor -> major: the
  original called it latent, but the CASINO_TILES branch is reachable from
  ordinary build commands in 5-6 player games (no supply or colour limit
  in build()), panicking every render in debug builds.
- F26 (sushi-go placings tiebreak) premise refuted: the official Sushi Go
  rulebook does specify most-puddings as the score tiebreaker, so the
  implementation is correct; residual is only that RULES.md omits it.
- F36 (modern-art all-cards payout) downgraded major -> minor: the crate's
  own RULES.md:102-105 explicitly documents cumulative payout "even if the
  artist didn't place this round". Still flagged for the pending cross-unit
  port-parity-vs-official-rules adjudication, since the doc may simply
  canonize the Go port's deviation. F37 (zero-card artists ranked) stays
  major: unlike F36 it is NOT documented in RULES.md.
- F15 (jaipur round starter) downgraded major -> minor on evidence basis
  only: every code claim verified, but the quoted "loser starts" rule has
  no in-repo source (RULES.md is a stub) and could not be corroborated;
  restore major if the rulebook quote is confirmed.

Evidence strengthenings recorded in the raw dumps: F2's nondeterminism
also affects the NUMBER of RNG draws via the recursive re-tie pass; F37's
counts map is seeded with zero entries (lib.rs:310-313), closing the gap
in the original's argument; F41's final screen additionally shows a bogus
"Current bid: $0" line; F34/F35 share one missing invariant and merit a
combined fix; F37's phantom artist values compound F36's payouts.

Overall assessment: accurate on code facts — all Go-parity line citations
checked out and the modern-art critical/major cluster is real and legally
reachable. Weaker than batch-c on rules adjudication: one finding rested
on an untraced setup path (F13, rejected), one on a wrong official-rules
premise (F26), and two majors needed severity moves in opposite directions
(F4 up, F36 down), leaving 6 majors instead of 8.
