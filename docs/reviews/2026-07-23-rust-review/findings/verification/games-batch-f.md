# Verification: games-batch-f (unit 8)

Independent verification of `findings/games-batch-f.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Crates: `game/zombie-dice-2`, `game/battleship-2`, `game/for-sale-2`,
`game/category-5-2`, `game/greed-2`, `game/farkle-2`, `game/tic-tac-toe-2`,
`game/no-thanks-2`, `game/liars-dice-2`. Raw verdict dumps:
`raw/games-batch-f-zd-bs.md`, `raw/games-batch-f-fs.md`,
`raw/games-batch-f-c5-greed.md`, `raw/games-batch-f-fk-ttt.md`,
`raw/games-batch-f-nt-ld.md`. Process log: `games-batch-f-LOG.md`.

Go sources: every crate except tic-tac-toe-2 has a brdgme-go original
(zombie_dice_1, battleship_1, for_sale_1, category_5_1, greed_1, farkle_1,
no_thanks_1, liars_dice_1), used for every port-parity claim. Claims whose
premise is an official rulebook not in the repo (SJG Zombie Dice, For Sale,
6 nimmt!, Farkle, No Thanks!, Perudo) are flagged external-basis, not
rejected.

## Per-finding verdicts

### zombie-dice-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | Cup draw order leaked to all players in PubState | major | CONFIRMED | pub_state clones cup verbatim (lib.rs:443); `take_dice` drains from the front (`drain(..n)`, :251) with shuffles only at turn start/refill, so the vec head genuinely is the next draw; Go PubState returned nil; DATA_DOCS.md:18 claims "no hidden information" (Lead re-read lib.rs:185-253) |
| F2 | Refill returns shotgun dice to the cup | minor | CONFIRMED (external basis) | Refill extends cup with ALL kept dice incl. shotguns (lib.rs:246-248); Go identical; RULES.md:31 documents it; only-brains-return premise is SJG rulebook knowledge |
| F3 | Rolloff state not exposed in PubState/render | minor | CONFIRMED | roll_off_players (lib.rs:173) never reaches PubState/pub_state()/render; only signal is the transient log |
| F4 | Panic paths on inconsistent deserialized state | minor | CONFIRMED | drain(..n), scores indexing, `% players` on 0, render scores[p] all trust start() invariants; Game is all-pub Deserialize; Go panics identically |
| F5 | Unbounded recursion on repeated busts | nit | CONFIRMED | roll -> next_player -> start_turn -> roll chain verified; fresh 3-die roll can bust, so consecutive busts recurse; theoretical |
| F6 | Rolloff tie announcement re-logged each wrap | nit | CONFIRMED | Block at lib.rs:276-285 unguarded. Fix caveat: a transition-only guard would miss legitimate membership changes mid-rolloff |
| F7 | Duplicated finish block in both command() arms | nit | CONFIRMED | Arms byte-identical except player_roll vs keep, ~15 lines |

### battleship-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F8 | shoot() drops Go's bounds check; panics on out-of-range Loc | minor | CONFIRMED | No check before boards[op][y][x] (lib.rs:314-317); Go validated (battleship.go:378-380); parser gating verified — shoot loc only from `Enum::exact(all_locations())` (command.rs:24, Lead re-read), so defense-in-depth as claimed |
| F9 | Indexing trusts lengths; inconsistent with .get() elsewhere | minor | CONFIRMED | Panicking indexes in is_finished/status/placings/place_ship vs defensive .get() in can_place/player_state/place_parser — all cited lines check out |
| F10 | expect("cell is a ship") in sunk branch | nit | CONFIRMED | lib.rs:331; unreachable today, future-variant panic risk |
| F11 | Ship::all() vs Direction::all() return-type asymmetry | nit | CONFIRMED | `&'static [Ship]` (:64) vs `Vec<Direction>` (:118). Fix caveat: command.rs:68 needs `.to_vec()` after the change |
| F12 | Hit-count helpers return i32 | nit | CONFIRMED | Both at :350/:362, fed to gen_placings i32 metrics |

### for-sale-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F13 | Selling-phase plays leaked via PubState.bids | major | CONFIRMED | play() stores into bids (lib.rs:258); pub_state clones bids unredacted (:412; field at :66, Lead re-read); RULES.md:23 says secret selection; Go ToPubState leaks identically; recommended redaction fix checked behavior-safe (Selling render reads only viewer's own bid; turn logic uses finished_bidding) |
| F14 | Passing pays floor(bid/2) vs official round-up | minor | CONFIRMED (external basis) | `/2` floors (lib.rs:233); Go identical; RULES.md:17 documents it |
| F15 | Deck/chip setup deviates from official | minor | CONFIRMED (external basis) | 15 chips flat, 20/20 decks {0,0,3..=20}, 3p-only removal all verified; Go identical |
| F16 | RULES.md cheque description factually wrong | minor | CONFIRMED | RULES.md:8 says 30 cheques with 2..=20; code has 20 with no 2s (lib.rs:89-91, Lead re-read); RULES.md:31 tie sentence contradicts placings() chips tie-break |
| F17 | End-of-game "scores" log shows cheque totals only | minor | CONFIRMED | Finished log renders cheque sums only (lib.rs:118); command() then appends a correct placings_log; Go identical |
| F18 | Phase inferred via SELL_THRESHOLD magic | minor | ADJUSTED (recommendation invalid; minor stands) | Mechanism confirmed, but the recommended `#[serde(default)] phase: Phase` migration is unsound: `Phase`'s `#[default]` is `Buying` (lib.rs:24-26, Lead re-read), so in-flight Selling games would deserialize as Buying and break turn logic. Sound fix: `Option<Phase>` with `current_phase()` fallback, or post-deserialize fixup |
| F19 | Empty-deck panic paths from corrupt state | nit | CONFIRMED | split_off underflow, remove(0), hands[p][0] all present; unreachable via legal play |
| F20 | Selling autoplay keys off player 0's hand | nit | CONFIRMED | Guard checks only hands[0] (lib.rs:151); `.all()` variant equivalent and also closes F19's :153 path |
| F21 | Dense -> standard-competition ranking vs Go | nit | CONFIRMED | Go GenPlacings dense (`curPlace++`) vs Rust gen_placings group-size increment; test :792-807 codifies [1,1,3] |
| F22 | render::highest_bid duplicate with different sentinel | nit | CONFIRMED | `best > 0` (render.rs:40-50) vs -1 (lib.rs:316-326); consistent today since real bids >= 1 |
| F23 | Helpers unnecessarily pub; player_state unchecked | nit | CONFIRMED | All helpers pub; player_state indexes chips/hands/cheques unchecked |

### category-5-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F24 | Player cap 8 vs Go/official 2-10 and own RULES.md | minor | CONFIRMED | MAX_PLAYERS=8 (lib.rs:21) vs Go game.go:58 (2-10) and RULES.md:3 "2-10"; deck supports 10 exactly |
| F25 | draw_cards unbounded recursion | minor | CONFIRMED | Termination trace verified: n > deck+discard leaves both piles empty with remaining > 0, zero progress per frame -> stack overflow; n == deck+discard terminates |
| F26 | expect calls in resolve/choose paths | nit | CONFIRMED | All three at :178/:235/:309; corrupt-state-only |
| F27 | Negative "points until end" after game ends | nit | CONFIRMED | render.rs:118 unconditional; PubState.finished exists but unused there |
| F28 | points() returns lower-is-better totals | nit | ADJUSTED (question discharged; nit stands) | The framework contract the finding asked to verify is now verified: ELO uses `place` from game_players (web/src/db.rs:1536-1548, Lead re-read), never points() — no backwards ratings. Residual is display/bot-prompt labeling, where raw bullheads are correct. The contingent fix (negate points()) would be a regression |
| F29 | Card(pub u8) permits invalid cards | nit | CONFIRMED | Pub tuple field + serde, no range check; command-unreachable |
| F30 | Test comment typo | nit | CONFIRMED | lib.rs:592 as described |
| F31 | hands[0] proxy in resolve_plays | nit | CONFIRMED | :228; uniform-size invariant holds by construction |

### greed-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F32 | score() ignores player arg, no turn validation | minor | CONFIRMED | `let _ = player;` at :295, no gate; parser-gated today; recommended guard verified safe for done()'s internal call path |
| F33 | E/e score-token collision | minor | CONFIRMED | UniCase Token (parser/mod.rs:51), first-Ok OneOf (:465-477), E1 triple before E2 in SCORES; pinned by test; Go parity |
| F34 | Length invariants unchecked on deserialized state | minor | CONFIRMED (detail corrected) | Panic sites :363/:366/:375 and render.rs:80 real; lib.rs:522 is a mis-citation — points() iterates and cannot panic (Lead re-read :521-523) |
| F35 | Duplicated placings-log block | nit | CONFIRMED (detail corrected) | Verbatim duplicates; actual ranges :477-485 and :498-506 |
| F36 | Theoretical i32 overflow | nit | CONFIRMED | Plain adds at :307/:366; port narrows Go's 64-bit int, still unreachable |
| F37 | E1 Foreground vs RULES.md black | nit | CONFIRMED | lib.rs:64 Foreground vs RULES.md:18 "black" vs Go greed.go:62 render.Black |

### farkle-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F38 | Scoring table duplicated in render.rs | minor | CONFIRMED | scoring_table() (render.rs:24-46) hardcodes what SCORES (lib.rs:47-82) owns; fix valid with row-order caveat |
| F39 | score() pub, ignores player arg | nit | CONFIRMED (strengthened) | `let _ = player;` at :245-246, siblings validate; Go Score ALSO ignores player — parity verified, in-crate inconsistency stands |
| F40 | Stale turn_score/remaining_dice in finished pub_state | nit | CONFIRMED (nuance) | Reset lives only in start_turn(), skipped when finished; on finishing `done` the score was banked (display stale, not lost); only finishing bust shows lost points; Go identical |
| F41 | Test sets out-of-range current_player | nit | CONFIRMED | lib.rs:614 unclamped `first_player + 1`; `% 3` fix valid |
| F42 | render.rs u8 vs Die alias | nit | CONFIRMED | render.rs:6/:13 vs `pub type Die = u8` (lib.rs:25) |
| F43 | Simplified Farkle variant cross-reference | nit | CONFIRMED (external basis) | SCORES + WIN_SCORE 5000 match Go exactly; RULES.md accurate; published-rules comparison external |
| F44 | PubState renderer indexes scores[p] unchecked | nit | CONFIRMED | render.rs:71-78 over 0..players |

### tic-tac-toe-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F45 | `1 - start_player` usize underflow on crafted state | minor | CONFIRMED | render.rs:34 (Lead re-read) vs safe lib.rs:141; grep confirms no Cargo.toml sets overflow-checks, so release wraps silently as claimed; fix needs one added import |
| F46 | Crafted `players` drives unbounded alloc/iteration | minor | CONFIRMED | gamer.rs:28/37/41/45 deserialize Game verbatim; renders() loops 0..player_count() (gamer.rs:70); placings/points sized by forged `players` |
| F47 | Dead misleading Cell::Empty arm in winner() | nit | CONFIRMED | matching_line (lib.rs:146-148) can never return Empty; the :142 arm is dead and would miscredit start_player if reachable |
| F48 | Mark casing inconsistent | nit | ADJUSTED (details; nit stands) | Inconsistency real but misstated: the "is X / is O" label is UPPERCASE (render.rs:36/:38, Lead re-read; pinned by exact-render test lib.rs:589-600), not lowercase. Actual split: log uppercase, label uppercase, board lowercase, RULES.md lowercase. Any fix must touch the exact-render test |

### no-thanks-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F49 | Vacuous test_init_player_chips | minor | CONFIRMED | Game::default() gives players==0; `for p in 0..0` never runs; assert unreachable (lib.rs:393-399) |
| F50 | Player cap 3-5 vs official 3-7 | minor | CONFIRMED (external basis, softened) | MIN/MAX 3-5, flat 11 chips, Go parity, RULES.md accurate all verified; note the original 2004 edition was officially 3-5 — "3-7" is later editions, so the deviation framing is edition-dependent |
| F51 | Unreachable "no chips" branch in pass() | nit | CONFIRMED | can_pass requires chips > 0; branch dead — meaning the intended helpful message is never shown, so folding (the finding's second option) beats deleting |
| F52 | Run-grouping duplicated lib/render | nit | CONFIRMED | lib.rs:156-176 and render.rs:23-42 line-for-line identical |
| F53 | Renderer panics on inconsistent PubState | nit | CONFIRMED (undercounts) | render.rs:77 unwrap, :91/:115 hands[p]; also chips[p]/final_scores[p] at :127/:129; lib.rs:275 same shape |

### liars-dice-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F54 | Turn after challenge goes past the caller, not to the loser | minor | CONFIRMED (external basis) | current_player never reassigned to the loser (lib.rs:208-211); Go call_command.go:67-68 identical; RULES.md line 37 documents it; loser-starts premise is Perudo rulebook knowledge |
| F55 | Index panics on inconsistent deserialized state | minor | CONFIRMED | All four locations assume player_dice.len()==players / valid bid_player; the :193 guard covers empty, not out-of-bounds |
| F56 | "fourty" typo preserved from Go | nit | ADJUSTED (reachability; nit stands) | Typo real (render.rs:104, Lead re-read; Go strings.go:135) but NOT "practically unreachable": the bid parser has no quantity cap (F57, command.rs:44-47, Lead re-read), so `bid 45 6` logs "fourty five" from ordinary input |
| F57 | Bid quantity uncapped in parser | nit | CONFIRMED | `max: None` at command.rs:44-48, no upper check in bid(); harmless (no arithmetic/allocation); suggested static cap never rejects a legal bid |
| F58 | Test gaps: redaction, wild-1 call, full game | nit | CONFIRMED | No test touches pub_state/player_state; no bid-value-1 call resolution; no play-to-completion; contract.rs generic only |

## Summary

- Findings verified: 58
- CONFIRMED: 54, ADJUSTED: 4 (F18, F28, F48, F56), REJECTED: 0,
  UNVERIFIABLE: 0. All 58 findings survive.
- Corrected tallies for the unit: 0 critical / 2 major / 22 minor /
  34 nit — identical to the original (the findings file's header tally
  matches the Lead's block-by-block recount; no severity changes).
- Lead spot-checked every ADJUSTED verdict directly against the snapshot
  (F18 Phase `#[default] Buying` at for-sale lib.rs:24-26; F28
  apply_rating_changes at web/src/db.rs:1525-1554; F48 render.rs:30-40
  label casing; F56 render.rs:104 + command.rs:44-47) and, for the
  all-confirm Worker (W1), re-verified the hardest confirmations: F1
  (zombie-dice lib.rs:185-253 cup ordering + drain semantics) and F8
  (battleship command.rs:17-29 parser gating).

## Notable corrections

- F18 (for-sale phase field): the finding stands but its recommended
  migration is unsound — a `#[serde(default)] phase: Phase` field
  defaults to `Phase::Buying`, so live games mid-Selling would
  deserialize as Buying and break whose-turn/can_play. A sound migration
  needs `Option<Phase>` with a `current_phase()` fallback or a
  post-deserialize fixup. Third recommendation-validity catch across the
  verified units (after unit 6's bug-inducing fix and unit 7's
  Cargo-invalid fix).
- F28 (category-5 points() contract): the "verify the framework
  contract" recommendation is discharged — ELO is placings-driven
  (web/src/db.rs:1536-1548 uses `place`, never points()), so
  lowest-wins games are rated correctly today; negating points() as the
  finding contemplated would be a display regression, not a fix.
- F48 (tic-tac-toe casing): premise partly wrong — the "is X / is O"
  label is uppercase, not lowercase; the real split is log+label
  uppercase vs board+RULES.md lowercase, and the exact-render test pins
  the current output.
- F56 (liars-dice "fourty"): the "practically unreachable" framing is
  contradicted by the batch's own F57 — the uncapped bid parser makes
  quantities >= 40 ordinary player input.
- F50 (no-thanks player cap) softened: the implemented 3-5 matches the
  original 2004 official edition; "official 3-7" is later editions.

Evidence strengthenings recorded in the raw dumps: F39 is Go parity
(Go's Score also ignores its player arg), sharpening it to an in-crate
consistency nit; F34's lib.rs:522 citation is wrong (points() iterates,
cannot panic) while the other three sites are real; F53 undercounts
(chips[p]/final_scores[p] at render.rs:127/:129 also unchecked); F45's
no-overflow-checks-in-release claim was grep-verified across the
workspace manifests; recommendation caveats: F6's transition-only guard
would miss mid-rolloff membership changes, F11's fix needs `.to_vec()`
at battleship command.rs:68, F38's derived table must preserve row order.

Overall assessment: a clean batch — zero rejections and zero severity
changes; the two majors (F1 zombie-dice cup-order leak, F13 for-sale
bids leak) both reproduced end-to-end including their Go-comparison
claims. The four adjustments are two detail/premise corrections
(F48, F56), one discharged verification question (F28), and one invalid
migration recommendation (F18) that matters for the remediation plan
rather than the finding itself.
