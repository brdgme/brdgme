# W5 verification: games-batch-f, no-thanks-2 + liars-dice-2 (F49-F58)

## no-thanks-2

### F49 Vacuous test test_init_player_chips — CONFIRMED
- evidence: game/no-thanks-2/src/lib.rs:393-399: `let mut g = Game::default();` — `Game` derives `Default`, so `players == 0`. `init_player_chips()` sets `player_chips = vec![STARTING_CHIPS; 0]` (empty). The loop `for p in 0..g.players` is `0..0`, body never executes, `assert_eq!(11, g.player_chips[p])` never runs. Test passes trivially regardless of `STARTING_CHIPS` or fill value.
- severity: agree (minor/quality) — vacuous test, no runtime impact.
- recommendation-check: valid — setting `g.players = 3` before the call makes the loop run 3 times; adding `assert_eq!(g.player_chips.len(), g.players)` closes the empty-vec hole. No bug introduced.

### F50 Player cap 3-5 vs official 3-7 — CONFIRMED (external basis, with edition caveat)
- evidence: lib.rs:18-20 `MIN_PLAYERS: usize = 3; MAX_PLAYERS: usize = 5; STARTING_CHIPS: i32 = 11;`; lib.rs:337-339 `player_counts() -> vec![3, 4, 5]`. Go parity confirmed: brdgme-go/no_thanks_1/no_thanks.go:34-35 `PlayerCounts() []int { return []int{3, 4, 5} }` and :223 `g.PlayerChips[p] = 11`. RULES.md line 3 and 8 accurately describe "3-5 player" and 11 chips.
- external basis: the "official 3-7 with scaled chips (11/9/7)" claim rests on the later (2017+) Amigo editions; the original 2004 edition was 3-5 players. So the implemented variant matches an official edition, not merely a house rule. Adjust the framing but the parity observation stands.
- severity: agree with minor at most; arguably nit given the 3-5 rule set is itself an official edition and RULES.md is accurate. Documented port-parity decision, no defect.
- recommendation-check: valid — "none required for parity; parameterise chips if 6-7p wanted" is correct and safe (11 for 3-5p, 9 for 6p, 7 for 7p per later editions).

### F51 Unreachable "no chips" branch in pass() — CONFIRMED
- evidence: lib.rs:92-96 `can_pass` requires `self.player_chips.get(player).copied().unwrap_or(0) > 0`; lib.rs:103-104 `pass` returns early `if !self.can_pass(player)`; therefore lib.rs:106-110 `if self.player_chips[player] <= 0 { ... "You have no chips left, you must take the card" }` is dead — a chipless player already got "can't pass at the moment". The style inconsistency (`.get().copied().unwrap_or(0)` at :94 vs direct index at :106/:111) is as described.
- severity: agree (nit/simplicity). Note a side effect worth flagging: because the branch is dead, the *helpful* message is never shown; chipless players get the generic error. That is a minor UX regression baked into the current structure (also present in Go).
- recommendation-check: valid — "fold the specific message into a single check" is the better half: check turn first, then chips with the specific message, preserving intent. Simply dropping the dead branch is behavior-preserving and also fine.

### F52 Run-grouping duplicated lib.rs/render.rs — CONFIRMED
- evidence: lib.rs:156-176 `player_hand_grouped` and render.rs:23-42 `group_sorted` are the identical run-detection algorithm (same `last == Some(c - 1)` / `std::mem::take` structure, line for line). The lib copy feeds scoring (`player_hand_score` :178-180); the render copy feeds display — a divergence would desync score vs display.
- severity: agree (nit/quality).
- recommendation-check: valid — a shared `fn group_runs(sorted: &[i32]) -> Vec<Vec<i32>>` is a straightforward extraction; render.rs already operates on a sorted slice, and `player_hand_grouped` can call it with `player_hand_sorted` output.

### F53 Renderer panics on inconsistent PubState — CONFIRMED
- evidence: render.rs:77 `pub_state.current_card.unwrap()` whenever `!pub_state.finished`; render.rs:91-92 and :115-118 index `pub_state.hands[p]` for `p in 0..pub_state.players` with no length check (also `chips[p]`/`final_scores[p]` at :127/:129 when finished — same shape); lib.rs:275 `chips: self.player_chips[player]` in `player_state`. All safe for server-generated state (`pub_state()` at lib.rs:243-269 always builds consistently), panic only on crafted/corrupt input.
- severity: agree (nit) — matches the cross-cutting deserialized-state-trust pattern noted across game crates; not reachable in normal operation.
- recommendation-check: valid — `if let Some(card)` / `.get(p)` hardening is safe; "acceptable as-is" is a reasonable stance.

## liars-dice-2

### F54 Turn after challenge goes to player after caller, not loser — CONFIRMED (external basis for official rule)
- evidence: game/liars-dice-2/src/lib.rs:208-211: after a call, `if !self.is_finished() { self.start_round(); self.current_player = self.next_active_player(self.current_player); }` — `current_player` is still the caller (never reassigned to `losing_player`, :173-191), so the player clockwise after the caller starts. Go identical: brdgme-go/liars_dice_1/call_command.go:67-68 `g.StartRound(); g.CurrentPlayer = g.NextActivePlayer(g.CurrentPlayer)`. RULES.md line 37 explicitly documents it: "The player who lost a die does not start the next round; the next active player (clockwise from the caller) starts." The official-Perudo loser-starts claim is external basis.
- severity: agree (minor/correctness) — deviation from the common official rule, but faithful port and honestly documented; the game is self-consistent.
- recommendation-check: valid — if aligning, set `current_player` to `losing_player` if still active else `next_active_player(losing_player)`, and update RULES.md line 37. The finding's "skipping eliminations" caveat correctly anticipates the losing player having just been eliminated.

### F55 Index panics on inconsistent deserialized state — CONFIRMED
- evidence: lib.rs:73-77 `active_players` and :79-83 `eliminated_player_list` index `self.player_dice[p]` for `p in 0..self.players`; :100-107 `roll_dice` same; :192-195 `self.player_dice[losing_player]` where `losing_player` may be `bid_player` (:173-176) — all panic if `player_dice.len() < players` or `bid_player >= player_dice.len()` in a corrupted stored game. (:193's `is_empty()` guard prevents remove-from-empty, not out-of-bounds.) `start()` (:241-248) builds consistent state, so unreachable in normal play.
- severity: agree (minor) — an HTTP-request panic from corrupt stored state is a real if low-probability failure mode, and it is systemic across game crates as noted.
- recommendation-check: valid — validate-on-load or `.get(p)` defensive access; low priority stance reasonable.

### F56 "fourty" typo preserved from Go — CONFIRMED (reachability ADJUSTED)
- evidence: game/liars-dice-2/src/render.rs:104 `4 => "fourty",` inside `number_str`; Go original brdgme-go/brdgme/strings.go:135 `tStr = "fourty"`. Faithful port of the typo.
- adjustment: "practically unreachable" is wrong — the parser has no quantity cap (F57, command.rs:44-48) and `bid()` never checks quantity against dice in play (lib.rs:119-139 only checks >= 1 and vs previous bid), so `bid 45 6` is accepted and logs "increased the bid to fourty five 6s". Reachable from ordinary (if silly) player input.
- severity: agree (nit/consistency).
- recommendation-check: valid — fix to "forty" in Rust; no parity reason to keep a typo, and the Rust `number_str` is already an intentional partial port (0..999 with digit fallback).

### F57 Bid quantity uncapped in parser — CONFIRMED
- evidence: command.rs:44-48 `Int { min: Some(MIN_BID_QUANTITY), max: None }`; lib.rs bid() has no upper-bound check either. No arithmetic overflow or allocation on the value (`matching` is counted independently, `number_str` falls back to digits for >= 1000 at render.rs:81-82). Harmless per rules — a bid above total dice is just an auto-lose bid.
- severity: agree (nit) — UX only.
- recommendation-check: valid with caveat — capping at `players * START_DICE_COUNT` in the parser would use the static max (30 for 6p) rather than the live dice count; that is fine (never rejects a legal bid, only absurd ones). Capping at the *current* total dice would be stricter and also legal. "Leave as-is" equally acceptable.

### F58 Test gaps: redaction, wild-1 bid value, full game — CONFIRMED
- evidence: the entire test module (lib.rs:362-419) contains only `start_works`, `example_round`, `player_elimination`; render.rs test module only `test_number_str`. (a) no test calls `pub_state()` or `player_state()` at all, so the hidden-dice redaction property is untested; (b) `example_round` bids values 5/6 only — the `*d as i32 == self.bid_value || *d == 1` condition (lib.rs:168) is exercised for wild-1s counting toward a non-1 bid (dice include 1s), but never with `bid_value == 1` itself; (c) no test drives a game to `Status::Finished`/placings. tests/contract.rs:1-7 is only the generic `assert_gamer_contract::<Game>()`, which cannot assert game-specific hidden-info semantics.
- adjustment (sub-point b, phrasing only): the wild-counting arm of the condition IS exercised by `example_round` (player 0 and 2 hold 1s, bid value 5, matching count includes them); what is untested is specifically a bid of value 1. The finding's parenthetical says this correctly; verdict unchanged.
- severity: agree (nit/quality).
- recommendation-check: valid — three small unit tests, no design risk.
