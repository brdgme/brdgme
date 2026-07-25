# Verification: texas-holdem-2 findings F1-F6

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5. Go: brdgme-review-snapshot/brdgme-go/texas_holdem_1.

## F1 (minor, correctness) - CONFIRMED

Claim: raise parser min bound diverges from Go and the "Go quirk preserved" comment is wrong.

Evidence:

Rust comment and bound, game/texas-holdem-2/src/command.rs:43-51:

```
/// Go quirk preserved: the `Int` bound's `min` is `g.LargestRaise`, not
/// `g.MinRaise()` (`max(MinimumBet, LargestRaise)`) - the raise action's
/// own validation (`Raise`/`raise` below) uses `MinRaise()` for both the
/// bound check and the error message, so the parser can accept an amount
/// the action then rejects. Preserved as-is per the porting correctness
/// rule; not fixed here.
fn raise_parser(&self, player: usize) -> impl Parser<T = Command> {
    let behind_current_bet = self.current_bet() - self.bets[player];
    let min = self.largest_raise;
```

Go parser, texas_holdem_1/command.go:172-175:

```
func (g *Game) RaiseParser(player int) brdgme.Parser {
	behindCurrentBet := g.CurrentBet() - g.Bets[player]
	min := g.MinRaise()
```

Go's parser min is `g.MinRaise()`, not `g.LargestRaise`. The comment's factual claim about Go is wrong, and the Rust bound diverges from Go. The real LargestRaise quirk is in Go's `CanRaise` (texas_holdem.go:328 `minRaise := g.LargestRaise`), which the Rust port preserves correctly at lib.rs:304-310 (`let min_raise = self.largest_raise;`).

Concrete pre-flop numbers verified: `minimum_bet = 10` (STARTING_MINIMUM_BET, lib.rs:30). In `new_hand` (lib.rs:590, 597) small blind bets 5 -> `bet()` (lib.rs:167-170) computes `raise_amount = 0 + 5 - 0 = 5`, so `largest_raise = 5`; big blind bets 10 -> `raise_amount = 0 + 10 - 5 = 5`, `largest_raise` stays 5. So parser min = 5 while `min_raise() = max(10, 5) = 10` (lib.rs:271-273). Parser accepts `raise 5`..`raise 9`; `raise()` (lib.rs:282-287) then rejects with "Your raise must be at least 10". In Go the parser itself would reject those amounts.

Severity: minor is right. User-facing inconsistency (parse-accept then action-reject) plus a behavioral divergence from Go in the error path only; no money/state corruption since `raise()` validates.

## F2 (minor, consistency) - CONFIRMED

Rust, lib.rs:32-33:

```
const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 8;
```

Used in `start` (lib.rs:654-660) to reject counts outside 2..=8.

Go, texas_holdem.go:58-59:

```
if players < 2 || players > 9 {
    return nil, errors.New("Texas hold 'em is limited to 2 - 9 players")
```

No comment anywhere near lib.rs:29-33 or `start` documents the 9 -> 8 divergence; the crate header (lib.rs:1) says "Rust port of `brdgme-go/texas_holdem_1`" and lib.rs:5-6 says "This module ports `texas_holdem.go` itself". PubState doc (lib.rs:57) states "2 through 8" but does not acknowledge Go allowed 9. Undocumented divergence confirmed.

## F3 (nit, consistency) - CONFIRMED

lib.rs:154-160:

```
fn bet_up_to(&mut self, player_num: usize, amount: i32) -> i32 {
    let bet_amount = amount.min(self.player_money[player_num]);
    self.bet(player_num, bet_amount)
        .expect("BetUpTo always bets an affordable amount");
```

Invariant holds: `bet()` only errors when `player_money < amount` (lib.rs:164-166), and `bet_amount` is clamped to `player_money` on the line above, so the expect cannot fire. Go equivalent panics too, texas_holdem.go:205-212:

```
func (g *Game) BetUpTo(playerNum int, amount int) int {
	betAmount := min(amount, g.PlayerMoney[playerNum])
	err := g.Bet(playerNum, betAmount)
	if err != nil {
		panic(err.Error())
	}
```

Technically request-reachable (bet_up_to runs from new_hand which runs from fold/showdown paths) so it brushes the CODING.md no-panic rule, but the invariant is locally provable and mirrors Go. Nit stands.

## F4 (nit, consistency) - CONFIRMED

Panics present as claimed:
- lib.rs:144 `assert!(!set.is_empty(), "No players in set");` - matches Go texas_holdem.go:193-194 `panic("No players in set")`.
- lib.rs:151 `panic!("Could not find any valid players");` - matches Go texas_holdem.go:202.
- card.rs:105-106 `if len < n { panic!("Not enough cards to pop"); }` - matches Go PopN's panic. All three carry doc comments saying they mirror Go (lib.rs:141-142, card.rs:102).

Call-site spot check for next_player_in_set:
- next_player (lib.rs:338-339): guarded by `if !betting_players.is_empty()`.
- new_betting_round (lib.rs:544-548): guarded by `if !self.betting_players().is_empty()`, else branch avoids the call.
- new_hand (lib.rs:577, 588, 596): `folded_players` is reset at lib.rs:557 before any call, so active_players == remaining_players, which is nonempty in every new_hand entry path (start with players >= 2; fold path has 1 active; showdown path only when !is_finished, i.e. remaining >= 2).
- showdown (lib.rs:486): next_remaining_player_num_from only reached when a pot existed, so remaining is nonempty.

pop_n deck math: new_hand deals 2 cards to each of at most MAX_PLAYERS=8 active players (16), community cards total 3+1+1=5 (flop/turn/river via new_community_cards, lib.rs:537-541); 21 <= 52. Unreachable from crafted input. Confirmed as nit.

## F5 (nit, simplicity) - CONFIRMED

poker.rs:31 `pub category: Option<Category>,` while the enum itself has a `None` variant (poker.rs:17). Both cited unwraps present:
- poker.rs:39 `let mut score = vec![self.category.unwrap_or(Category::None) as i32];`
- poker.rs:54 `(res.category.unwrap_or(Category::None) < Category::StraightFlush`

`Option<Category>` duplicates `Category::None`; `Category` would collapse the unwraps (the poker.rs:61 `res.category.is_some()` check would become `!= Category::None`). Nit confirmed.

## F6 (nit, quality) - CONFIRMED

`command()` (lib.rs:698-806) has exactly five match arms (AllIn, Call, Check, Fold, Raise). Each arm repeats the identical block

```
if self.is_finished() {
    let scores: Vec<(usize, i32)> = (0..self.players)
        .map(|p| (p, self.player_total_money(p)))
        .collect();
    logs.push(placings_log(&self.placings(), Some(&scores)));
}
Ok(CommandResponse {
    logs,
    can_undo: ...,
    remaining_input: remaining.to_string(),
})
```

at lib.rs:720-730 (AllIn), 738-748 (Call), 756-766 (Check), 774-784 (Fold), 792-802 (Raise). Only differences: the action called (lib.rs:719, 737, 755, 773, 791) and `can_undo` - false at 728, 746, 764, 782; true at 800 (Raise). Count, span (719-803), and can_undo claim all verified.

## Summary

| ID | Verdict | Severity |
|----|---------|----------|
| F1 | CONFIRMED | minor |
| F2 | CONFIRMED | minor |
| F3 | CONFIRMED | nit |
| F4 | CONFIRMED | nit |
| F5 | CONFIRMED | nit |
| F6 | CONFIRMED | nit |
