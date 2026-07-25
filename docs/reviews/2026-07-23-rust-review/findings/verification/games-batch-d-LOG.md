# Verification LOG: games-batch-d (2026-07-24)

Independent verification of `findings/games-batch-d.md` (unit 6, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

46 findings total in games-batch-d.md, numbered F1-F46 in document order.
Four serial Workers (model fable per user override), split by crate so each
reads a coherent source set:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | game/lords-of-vegas-1 | F1 unimplemented!() in command dispatch (major), F2 HashMap/HashSet nondeterminism feeds boss-tie RNG (major), F3 resolve_boss_ties empty logs (minor), F4 usize underflow in renderer supplies (minor), F5 Loc::parse_str out-of-range lots / neighbours underflow (minor), F6 lazy_static vs OnceLock (minor), F7 unreachable! in starting-cash fold (nit), F8 serde_json runtime dep only used in tests (nit), F9 redundant FromIterator import (nit), F10 hardcoded 3 vs BLOCK_WIDTH (nit), F11 casino colours vs RULES.md (nit), F12 player counts 2-6 vs official 2-4 (nit) | raw/games-batch-d-lov.md |
| W2 | game/jaipur-2 | F13 8 camels vs official 11 (major), F14 no bonus token for 6/7-card sale (major), F15 next-round starter not round loser (major), F16 camel token counted as bonus token in tie-break (minor), F17 RULES.md one-line stub (minor), F18 mixed-type sell silently becomes sell-N-of-first (minor), F19 dead is_empty branch in command_parser (nit), F20 unwrap_or(Good::Diamond) fallback (nit), F21 placings-log block duplicated (nit), F22 "N rounds remaining" overstates (nit), F23 camel display leaks exact-zero vs exact PubState (nit) | raw/games-batch-d-jaipur.md |
| W3 | game/sushi-go-2 | F24 round-2 passes right vs official always-left (minor), F25 pudding all-tied 2p awards nothing (minor), F26 placings pudding tiebreak undocumented (minor), F27 vacuous test_hand_passing_left (minor), F28 draw_count dead (2,9) entry + unwrap_or(9) (minor), F29 pudding hint "least -6" false in 2p (nit), F30 maki second-place <=3 guard (nit), F31 render_name duplicated (nit), F32 dummy-slot guard order (nit), F33 command() placings-log duplication (nit) | raw/games-batch-d-sushigo.md |
| W4 | game/modern-art-2 | F34 infinite busy-loop when all hands empty after settle (critical), F35 round 4 can start on empty-handed player, soft-lock (major), F36 payout pays all purchases incl. non-top-3 artists (major), F37 zero-card artists ranked and awarded (major), F38 unreachable!/unchecked indexing in round_cards (minor), F39 RULES.md winner-takes-next-turn wrong (minor), F40 RULES.md double-auction omits Once Around (minor), F41 stale State::Auction on game end (minor), F42 "Current bid: $0" before any bid (nit), F43 sealed/once-around ties favor auctioneer (nit), F44 can_add throwaway vec![] (nit), F45 guarded bid.unwrap() (nit), F46 redundant use std::default::Default (nit) | raw/games-batch-d-ma.md |

Note: the findings file's own summary line says "1 critical, 9 major,
14 minor, 22 nit = 46"; recount of the per-finding severity fields gives
1 critical / 8 major / 16 minor / 21 nit. To be settled at curation from
the verified severities.

Game-rule correctness claims are judged against each crate's rules docs /
in-code comments and, where a Go source exists (sushi_go_1 and modern_art_1
in brdgme-go/), against the Go original; lords-of-vegas-1 and jaipur-2 have
no Go source in the snapshot, so official-rulebook claims are checked for
internal consistency and flagged if they rest solely on the original
reviewer's rules knowledge. Lead spot-checks all REJECTED/ADJUSTED
verdicts; if a Worker confirms everything, Lead re-verifies its 1-2 hardest
confirmations. Curated report: verification/games-batch-d.md.

### W1 dispatched — lords-of-vegas-1 (F1-F12)

### W1 returned

10 CONFIRMED, 2 ADJUSTED. Dump: raw/games-batch-d-lov.md.
- F2 strengthened: order affects which tile gets which roll AND (via the
  recursive re-tie pass) the total number of RNG draws; Loc derives Ord
  (board.rs:73) so the recommended fix is feasible.
- F4 ADJUSTED (minor -> major): dice/token underflows latent as claimed,
  but the CASINO_TILES underflow (render.rs:117) is reachable in normal
  5-6p play — build() imposes no supply/colour limits, so 10+ same-colour
  builds push casino_tile_count past CASINO_TILES=9.
- F7 ADJUSTED (details only, nit stands): min GameEnd insert position is
  38 (2p), not 39; card.rs:27-29 DOES carry the invariant comment — the
  missing comment is at lib.rs:118.
- F12 confirmed with evidence basis flagged external (RULES.md silent on
  player count; no Go source).
- No brdgme-go lords_of_vegas source in the snapshot (checked).

### Lead spot-checks (W1)

- F4 ADJUSTED upheld, minor -> major — read lib.rs:251-314 (build()
  validates only turn/location/ownership/cash; casino colour is free and
  unlimited), lib.rs:29 (CASINO_TILES = 9), render.rs:117
  (`CASINO_TILES - self.board.casino_tile_count(*casino)`). With 5-6
  players (10-12 owned starting lots) all building one colour the count
  exceeds 9; debug builds panic on subtract overflow at every render.
- F7 ADJUSTED upheld, nit stands — read card.rs:20-35: comment at
  card.rs:27-29 states the last-quarter invariant; 2p math gives
  quart_pile = (48-4)/4 = 11, min insert position 48-10 = 38.

### W2 dispatched — jaipur-2 (F13-F23)

### W2 returned

9 CONFIRMED, 1 ADJUSTED (F15), 1 REJECTED (F13).
Dump: raw/games-batch-d-jaipur.md.
- F13 REJECTED: card_count is the post-setup composition — start_round
  conjures the 3 market camels out of thin air (lib.rs:223), so total
  camels = 8+3 = 11 and the 52-card deck equals the official 55-card game
  after market seeding; deck ends at 40 as official. The finding's
  recommended fix (11 in card_count) would create 14 camels — a bug.
- F15 ADJUSTED: all code facts confirmed (no path sets the next-round
  starter; sell-ending and deck-exhaustion endings leave the acting
  player to start), but the "official rule: loser starts" premise is
  external-only and uncorroborated in-repo; severity major -> minor
  pending rulebook adjudication.
- F14 upheld as the batch's genuine scoring defect (bonus keys 3/4/5
  only; render.rs:153 "5 or more" and DATA_DOCS.md:13 "3+" contradict).
- F20 strengthened: Many::some_spaced min=Some(1)
  (lib/game parser/mod.rs:310-317, errors below min at 382-391).
- No Go jaipur source in the snapshot (checked).

### Lead spot-checks (W2)

- F13 REJECTED upheld — read lib.rs:97-107 (card_count, Camel => 8) and
  lib.rs:213-244 (start_round): deck = 52 cards incl. 8 camels; market
  seeded `vec![Camel; 3]` NOT drawn from the deck; replenish to 5 draws
  2; hands 2x5 -> deck 40. Official: 55 - 3 market camels - 10 dealt - 2
  replenish = 40. Identical in-play distribution (11 camels total); the
  original reviewer read the table without tracing setup.
- F15 ADJUSTED accepted — behavioral claims verified by W2 line-by-line;
  the rule premise has no in-repo source (RULES.md is a stub, F17), so
  per charter the severity cannot rest on an unverifiable external
  quote. Recorded minor with note: restore major if the rulebook quote
  is confirmed at adjudication.

### W3 dispatched — sushi-go-2 (F24-F33)

### W3 returned

8 CONFIRMED, 1 ADJUSTED (F31), 1 REJECTED (F26).
Dump: raw/games-batch-d-sushigo.md.
- F26 REJECTED by worker: all code/test/Go/RULES.md facts check out, but
  official Gamewright rules DO specify most-puddings as the points
  tiebreaker (worker model knowledge, high confidence), so the
  implementation is correct; residual is only the RULES.md omission.
- F31 ADJUSTED (details only, nit stands): duplication is logic-level,
  not "byte-for-byte" (method vs free fn); underflow expression
  confirmed in both copies.
- F28 precision note: draw_count(2) IS called by a test (lib.rs:1301);
  the (2,9) entry is dead in production only.
- Every "inherited from Go port" claim held (near line-for-line port of
  brdgme-go/sushi_go_1).

### Lead spot-checks (W3)

- F26 reviewed — grepped the crate's RULES.md: no tiebreaker documented
  (only maki/pudding scoring lines), confirming the code-fact side. The
  overturn rests on official-rules knowledge; Lead concurs with the
  worker that the official rulebook contains "if there is a tie, the
  player with the most pudding cards wins", refuting the finding's core
  premise ("published rules specify no tiebreaker"). RECONCILED: per
  batch-c precedent (F23), premise-refuted-but-residual-survives is
  recorded as ADJUSTED (minor/correctness -> nit/consistency: document
  the pudding tiebreak in RULES.md), not REJECTED.
- F31 ADJUSTED accepted on W3's quoted spans; nit stands either way.

### W4 dispatched — modern-art-2 (F34-F46)

### W4 returned

11 CONFIRMED, 2 ADJUSTED (F36, F45). Dump: raw/games-batch-d-ma.md.
- F34 critical upheld: skip loop lib.rs:452-459 unguarded, log per
  iteration, legally reachable in round 4; Go :689-695 identical.
- F35 major upheld: no skip on the round transition; empty Enum::exact
  play parser; no pass outside auctions; Go :432-434 identical.
- F36 ADJUSTED (major -> minor): facts and Go parity confirmed, but
  RULES.md:102-105 explicitly documents the behavior ("even if the
  artist didn't place this round") — documented-in-crate precedent;
  the doc may canonize an inherited defect (adjudication pending).
- F37 major upheld with a key trace the original skipped: counts map is
  seeded for ALL suits including zeros (lib.rs:310-313), so the -1
  sentinel really does rank zero-card artists; NOT documented in
  RULES.md.
- F41 strengthened: final screen also shows "Current bid: $0 by
  <auctioneer>" in addition to the stale "is auctioning" line.
- F45 ADJUSTED (details only, nit stands): real expression is
  `(bid.is_none() || *bid.unwrap() > 0)` — the original quote dropped
  `> 0` and would not compile.
- The review's Go line citations were accurate throughout.

### Lead spot-checks (W4)

- F36 ADJUSTED upheld, major -> minor — read RULES.md:96-105: "Every
  player is then paid, for **each** card they've purchased so far this
  round, the *total* cumulative value of that card's artist - even if
  the artist didn't place this round." Documented intended in-crate
  behavior; kept minor (not nit) because it is a material economy
  deviation from official Modern Art rules awaiting the cross-unit
  port-parity-vs-official adjudication.
- F45 ADJUSTED accepted on the dump's quoted expression; nit stands.

## Curation complete (2026-07-24)

38/46 CONFIRMED, 7 ADJUSTED (F4 minor->major; F7 details; F15
major->minor; F26 minor->nit; F31 details; F36 major->minor; F45
details), 1 REJECTED (F13 jaipur camel count — market camels are
conjured at setup, composition matches official). Corrected unit tally:
1 critical / 6 major / 16 minor / 22 nit over 45 surviving findings
(original per-finding fields: 1/8/16/21 over 46; the findings file's own
summary line "1 critical, 9 major, 14 minor, 22 nit" miscounts its own
blocks). Report: verification/games-batch-d.md. LOG closed.
