# Verification LOG: games-batch-f (2026-07-24)

Independent verification of `findings/games-batch-f.md` (unit 8, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

58 findings total in games-batch-f.md, numbered F1-F58 in document order.
Lead recount from the blocks: 0 critical / 2 major (F1 zombie cup order,
F13 for-sale bids leak) / 22 minor / 34 nit = 58 — matches the file's own
header tally line. Five serial Workers (model fable per user override),
split by crate so each reads a coherent source set:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | game/zombie-dice-2 + game/battleship-2 | F1 cup draw order leaked in PubState (major), F2 refill returns shotguns (minor), F3 rolloff state not in PubState (minor), F4 panic paths on deserialized state (minor), F5 unbounded bust recursion (nit), F6 rolloff tie re-logged (nit), F7 duplicated finish block (nit); F8 shoot() drops bounds check (minor), F9 inconsistent indexing vs .get() (minor), F10 expect in sunk branch (nit), F11 all() return-type asymmetry (nit), F12 i32 hit counts (nit) | raw/games-batch-f-zd-bs.md |
| W2 | game/for-sale-2 | F13 selling plays leaked via PubState.bids (major), F14 pass pays floor(bid/2) (minor), F15 deck/chip setup deviates (minor), F16 RULES.md cheque desc wrong (minor), F17 end log shows cheques only (minor), F18 phase inferred via SELL_THRESHOLD (minor), F19 empty-deck panic paths (nit), F20 autoplay keys off hands[0] (nit), F21 dense vs standard ranking (nit), F22 render highest_bid duplicate (nit), F23 helpers pub / player_state unchecked (nit) | raw/games-batch-f-fs.md |
| W3 | game/category-5-2 + game/greed-2 | F24 player cap 8 vs 2-10 (minor), F25 draw_cards unbounded recursion (minor), F26 expect calls in resolve/choose (nit), F27 negative "points until end" (nit), F28 points() lower-is-better (nit), F29 Card(pub u8) unvalidated (nit), F30 test comment typo (nit), F31 hands[0] proxy in resolve_plays (nit); F32 score() ignores player arg (minor), F33 E/e token collision (minor), F34 length invariants unchecked (minor), F35 duplicated placings-log block (nit), F36 i32 overflow theoretical (nit), F37 E1 Foreground vs RULES.md black (nit) | raw/games-batch-f-c5-greed.md |
| W4 | game/farkle-2 + game/tic-tac-toe-2 | F38 scoring table duplicated in render (minor), F39 score() pub ignores player (nit), F40 stale turn_score in finished pub_state (nit), F41 test sets out-of-range current_player (nit), F42 u8 vs Die alias (nit), F43 simplified Farkle variant cross-ref (nit), F44 PubState renderer scores index (nit); F45 `1 - start_player` underflow (minor), F46 crafted players unbounded alloc (minor), F47 dead Cell::Empty arm (nit), F48 mark casing inconsistent (nit) | raw/games-batch-f-fk-ttt.md |
| W5 | game/no-thanks-2 + game/liars-dice-2 | F49 vacuous test_init_player_chips (minor), F50 player cap 3-5 vs 3-7 (minor), F51 unreachable no-chips branch (nit), F52 run-grouping duplicated (nit), F53 renderer panics on crafted PubState (nit); F54 turn after challenge (minor), F55 index panics on deserialized state (minor), F56 "fourty" typo (nit), F57 bid quantity uncapped (nit), F58 test gaps (nit) | raw/games-batch-f-nt-ld.md |

Go sources: all crates except tic-tac-toe-2 have brdgme-go originals
(zombie_dice_1, battleship_1, for_sale_1, category_5_1, greed_1, farkle_1,
no_thanks_1, liars_dice_1); port-parity claims are checked against them.
tic-tac-toe-2 has no Go source. Claims resting solely on external official-
rulebook knowledge (SJG Zombie Dice, For Sale, 6 nimmt!, Farkle, No Thanks!,
Perudo) are flagged external-basis, not rejected outright. Recommended
fixes are themselves checked for validity (prior units found a bug-inducing
fix in unit 6 and a Cargo-invalid fix in unit 7).
Lead spot-checks all REJECTED/ADJUSTED verdicts; if a Worker confirms
everything, Lead re-verifies its 1-2 hardest confirmations. Curated report:
verification/games-batch-f.md.

### W1 dispatched — zombie-dice-2 + battleship-2 (F1-F12)

### W1 returned

12 CONFIRMED, 0 ADJUSTED/REJECTED. Dump: raw/games-batch-f-zd-bs.md.
- F1 major upheld: pub_state clones cup verbatim (lib.rs:443); drain(..n)
  from the front makes the vec head the next draw; shuffle only at turn
  start/refill; Go PubState returned nil; DATA_DOCS.md:18 claims "no
  hidden information".
- F8 parser gating verified: Enum::exact(all_locations()) only yields
  in-bounds y,x; Go validated at battleship.go:378-380.
- F2 official-rules premise marked external basis (SJG rulebook not in
  repo); Go parity + RULES.md:31 verified.
- Recommendation caveats recorded: F6 transition-only guard would miss
  membership changes mid-rolloff; F11 fix needs `.to_vec()` at
  command.rs:68. Both nits stand with caveats.

### Lead spot-checks (W1)

All-confirm Worker, so Lead re-verified the two hardest directly:
- F1 — read lib.rs:185-253: `pub cup: Vec<Dice>` doc-commented "in draw
  order" (:193-194); `take_dice` drains from the front (`drain(..n)`,
  :251) and only `shake_cup` at turn start/refill randomizes — vec head
  is genuinely the next draw. Upheld, major stands.
- F8 — read command.rs:17-29: shoot loc comes only from
  `Enum::exact(all_locations())`, so the missing bounds check in shoot()
  is defense-in-depth as claimed. Upheld, minor stands.

### W2 dispatched — for-sale-2 (F13-F23)

### W2 returned

10 CONFIRMED, 1 ADJUSTED (F18: finding stands, recommendation invalid).
Dump: raw/games-batch-f-fs.md.
- F13 major upheld: play() stores into bids (lib.rs:258), pub_state
  clones bids unredacted (:412), RULES.md:23 says secret selection; Go
  ToPubState leaks identically; the recommended redaction fix checked
  safe (Selling render reads only the viewer's own bid; turn logic uses
  finished_bidding).
- F18 ADJUSTED: SELL_THRESHOLD mechanism confirmed, but the recommended
  `#[serde(default)] phase: Phase` migration is unsound — `Phase`'s
  `#[default]` is `Buying`, so in-flight Selling games would deserialize
  as Buying and break turn logic. Sound fix: `Option<Phase>` with
  `current_phase()` fallback or post-deserialize fixup. Minor stands.
- F16/F21 verified in detail (RULES.md misdescription; Go GenPlacings
  dense `curPlace++` vs Rust standard-competition group-size increment).

### Lead spot-checks (W2)

- F18 ADJUSTED upheld — Lead read lib.rs:22-29: `Phase` derives
  `Default` with `#[default]` on `Buying`; a serde-defaulted field would
  mislabel mid-Selling games. Recommendation-invalid correction correct.
- F13 partially re-verified while in the file: `pub bids: Vec<i32>` is a
  PubState field (lib.rs:66) and cheque_deck() is `{0,0,3..=20}` of 20
  (lib.rs:89-91), also corroborating F16. Upheld.

### W3 dispatched — category-5-2 + greed-2 (F24-F37)

### W3 returned

13 CONFIRMED, 1 ADJUSTED (F28). Dump: raw/games-batch-f-c5-greed.md.
- F28 ADJUSTED (nit stands, question discharged): the framework contract
  the finding asked to verify is now verified — ELO ratings are driven by
  `place` from game_players (web/src/db.rs:1536-1548), never `points()`,
  so no backwards ratings for lowest-wins games. Residual is display/bot-
  prompt labeling only, where raw bullheads are correct. Important: the
  finding's contingent fix (negate points()) would be a regression.
- F25 recursion-termination trace verified precisely (zero progress per
  frame when n > deck+discard; terminates at equality).
- F33 parser mechanics verified (UniCase Token parser/mod.rs:51,
  first-Ok OneOf :465-477, E1-first SCORES order, pinned test).
- Detail corrections inside CONFIRMED verdicts: F34's lib.rs:522 is a
  mis-citation (points() iterates, cannot panic; other three sites
  real); F35's true ranges are :477-485 and :498-506.

### Lead spot-checks (W3)

- F28 ADJUSTED upheld — Lead read web/src/db.rs:1525-1554:
  apply_rating_changes selects `place` per player and never touches
  points(); ELO is placings-driven. Also greed-2 lib.rs:521-523 confirms
  the F34 mis-citation (pure iteration). Nit stands with the discharged
  recommendation recorded.

### W4 dispatched — farkle-2 + tic-tac-toe-2 (F38-F48)

### W4 returned

10 CONFIRMED, 1 ADJUSTED (F48 detail, nit stands).
Dump: raw/games-batch-f-fk-ttt.md.
- F48 ADJUSTED: inconsistency real, but the "is X / is O" label is
  UPPERCASE (render.rs:36,38; pinned by the exact-render test at
  lib.rs:589-600), not lowercase as the finding claimed. Actual split:
  log uppercase, label uppercase, board lowercase, RULES.md lowercase.
  Any casing fix must touch the exact-render test.
- F45 verified both halves: render.rs:34 `1 - start_player` vs safe
  lib.rs:141; grep confirms no Cargo.toml sets overflow-checks, so
  release wraps silently as claimed.
- F46 verified end-to-end: gamer.rs:28/37/41/45 deserialize Game
  verbatim; renders() loops 0..player_count() (gamer.rs:70).
- F39 strengthened: Go Score also ignores the player arg (parity
  verified). F40 nuance: finishing `done` banks the score first (display
  stale only); only finishing bust shows lost points.

### Lead spot-checks (W4)

- F48 ADJUSTED upheld — Lead read render.rs:30-40: labels are
  `" is X, "` / `" is O"` (uppercase) beside lowercase board glyphs
  (:16-17); the finding's "label lowercase" premise was wrong; nit
  stands recast. Same read re-confirms F45's `1 - start_player` at :34.

### W5 dispatched — no-thanks-2 + liars-dice-2 (F49-F58)

### W5 returned

9 CONFIRMED, 1 ADJUSTED (F56 reachability, nit stands).
Dump: raw/games-batch-f-nt-ld.md.
- F56 ADJUSTED: typo real (render.rs:104; Go strings.go:135) but NOT
  "practically unreachable" — F57's own uncapped bid parser makes
  `bid 45 6` ordinary input, which logs "fourty five".
- F50 softened: original 2004 No Thanks! edition was officially 3-5;
  "official 3-7" is later editions. Minor stands, external basis.
- F54 verified vs Go call_command.go:67-68 and RULES.md:37.
- F53 undercounts: chips[p]/final_scores[p] at render.rs:127/:129 also
  unchecked.

### Lead spot-checks (W5)

- F56 ADJUSTED upheld — Lead read liars-dice render.rs:98-111 ("fourty"
  at :104) and command.rs:41-48 (`max: None` on quantity): a >= 40 bid
  is enterable by ordinary input; reachability correction correct.

## Curation complete (2026-07-24)

54/58 CONFIRMED, 4 ADJUSTED (F18 recommendation invalid; F28 question
discharged; F48 premise detail; F56 reachability), 0 REJECTED,
0 UNVERIFIABLE. All 58 findings survive. Corrected unit tally:
0 critical / 2 major / 22 minor / 34 nit — identical to the original
(header tally matches the Lead's block recount; no severity changes).
Report: verification/games-batch-f.md. LOG closed.
