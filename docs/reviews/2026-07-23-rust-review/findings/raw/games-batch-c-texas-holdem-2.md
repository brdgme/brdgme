# Raw findings: game/texas-holdem-2

Worker: games-batch-c-texas-holdem-2. Review scope: crate root
`/home/beefsack/Development/brdgme-review-snapshot/rust/game/texas-holdem-2/`
(src/lib.rs, src/poker.rs, src/card.rs, src/command.rs, src/render.rs,
src/bin/*). Review-only; findings incremental.

Covered with cross-reference to `/home/beefsack/Development/brdgme-review-snapshot/brdgme-go/texas_holdem_1/`
and `/home/beefsack/Development/brdgme-review-snapshot/brdgme-go/libpoker/`.

Findings below; worker summary at end.

### Raise parser min bound diverges from Go, and the "Go quirk preserved" comment is factually wrong
- severity: minor
- category: correctness
- location: game/texas-holdem-2/src/command.rs:41-51
- finding: The doc comment on `raise_parser` claims "Go quirk preserved: the `Int` bound's `min` is `g.LargestRaise`, not `g.MinRaise()`". This is false: Go's `RaiseParser` (`brdgme-go/texas_holdem_1/command.go:174`) uses `min := g.MinRaise()`. (The comment's author appears to have conflated it with Go's `CanRaise`, `texas_holdem.go:328`, which genuinely does use `g.LargestRaise` — that quirk is real and correctly preserved in `lib.rs:298-310`.) So the Rust parser uses `self.largest_raise` (line 51) where Go used `max(minimum_bet, largest_raise)`. Concrete effect: pre-flop after blinds, `largest_raise` is 5 (big blind over small blind) while `MinRaise()` is 10, so the parser accepts `raise 5`..`raise 9` which `Game::raise` (lib.rs:282-287) then rejects with "Your raise must be at least 10". Go rejected these at parse time. No state corruption (the action re-validates), but it is a real behavioural divergence from the source, and the incorrect comment will mislead anyone auditing port fidelity.
- recommendation: Change `let min = self.largest_raise;` to `let min = self.min_raise();` and rewrite the comment to state that the parser bound matches Go's `g.MinRaise()` (optionally noting the genuine `CanRaise`/`LargestRaise` quirk lives in `lib.rs`).

### Max player count is 8, Go original supports 9 — undocumented divergence
- severity: minor
- category: consistency
- location: game/texas-holdem-2/src/lib.rs:33
- finding: `MAX_PLAYERS: usize = 8`, so `Game::start` rejects 9 players and `player_counts()` returns 2..=8. The Go original allows 2-9 (`texas_holdem.go:58` "Texas hold 'em is limited to 2 - 9 players", and `PlayerCounts()` returns 2..=9). Nothing in the deck math requires 8 (9 players needs only 18 hole + 5 community cards). If the reduction is deliberate (e.g. UI constraints) it is undocumented — there is no comment, and the crate doc header claims a straight port of `texas_holdem_1`.
- recommendation: Either restore `MAX_PLAYERS = 9` for Go parity, or add a short comment documenting why the port deliberately caps at 8.

### `bet_up_to` uses `.expect()` in a runtime path
- severity: nit
- category: consistency
- location: game/texas-holdem-2/src/lib.rs:158
- finding: `self.bet(player_num, bet_amount).expect("BetUpTo always bets an affordable amount")`. The invariant holds today (`bet_amount = amount.min(player_money)`), so the panic is unreachable, and Go panicked in the equivalent spot (`texas_holdem.go:205-212`). But docs/CODING.md says no `.expect()` in runtime paths; the invariant is also only locally true — a future edit to `bet_up_to` could make it reachable from blinds posting during `new_hand`.
- recommendation: Either restructure so `bet` cannot fail here (e.g. an infallible internal `bet_unchecked`-style helper mirroring the clamping), or keep as-is with the understanding it mirrors Go; at minimum it is a known style-rule exception worth a comment.

### Documented Go-mirroring panics in `next_player_in_set` and `pop_n`
- severity: nit
- category: consistency
- location: game/texas-holdem-2/src/lib.rs:144
- finding: `next_player_in_set` has `assert!(!set.is_empty(), "No players in set")` (lib.rs:144) and a trailing `panic!("Could not find any valid players")` (lib.rs:151); `card::pop_n` panics `"Not enough cards to pop"` (card.rs:106). All three mirror Go panics and are documented as such. I traced every call site: `next_player_in_set` is only invoked with non-empty sets guarded by callers (`next_player` checks `betting_players` non-empty, `new_betting_round` guards, `new_hand`/`fold`/`showdown` guarantee remaining/active non-empty, and the second panic is unreachable since the loop covers all players); `pop_n` underflow is impossible (52-card deck, <= 8 players * 2 hole cards + 5 community = 21 max). Not reachable from crafted player input, so no service-panic risk — noted only because CODING.md discourages panics in runtime paths.
- recommendation: No change required for correctness; if the panic-free rule is ever enforced strictly, convert to `Result`/debug_assert. Low priority.

### `HandResult.category: Option<Category>` is redundant with the `Category::None` variant
- severity: nit
- category: simplicity
- location: game/texas-holdem-2/src/poker.rs:31
- finding: `Category` has a `None` variant (mirroring Go's `CATEGORY_NONE = 0` iota value, needed for `hand_score` to produce 0), yet `HandResult.category` wraps it in `Option`, forcing `unwrap_or(Category::None)` at both use sites (poker.rs:39, 54). Two representations of "no category" in one type. A plain `category: Category` defaulting to `Category::None` (via `#[derive(Default)]` on the enum with `#[default]` on `None`) would be simpler and keep the same `as i32` scoring.
- recommendation: Drop the `Option`, default `HandResult.category` to `Category::None`.

### Placings-log block duplicated across all five `command()` arms
- severity: nit
- category: quality
- location: game/texas-holdem-2/src/lib.rs:719
- finding: The identical 8-line `if self.is_finished() { ... placings_log ... }` block plus the `Ok(CommandResponse { ... })` construction is copy-pasted into each of the five match arms (lib.rs:719-803), with the only differences being the action called and `can_undo` (false everywhere except Raise). Any future change to finished-game logging has to be made in five places.
- recommendation: Restructure to bind `(logs, can_undo)` per arm in a small match, then run the finished/placings logic and `CommandResponse` construction once.

## Worker summary

Covered, with full cross-reference against `brdgme-go/texas_holdem_1` (texas_holdem.go, command.go) and `brdgme-go/libpoker/hand.go`:

- Poker hand evaluation (`poker.rs`): all category detectors, `is_straight` including the wheel (ace-low) path, `find_multiple`, `find_highest_rank`, `cards_by_suit`/`cards_by_rank`, `winning_hand_result` tie-breaking, `hand_score` ordering vs Go iota values. Verified the ace-low straight condition cannot misfire on non-wheel 4-runs (the loop's reset-on-gap guarantees a trailing 4-run ends at rank 2). Considered clean.
- Betting logic (`lib.rs`): fold/check/call/raise/allin guards vs Go, min-raise semantics, `largest_raise` tracking, blind posting incl. heads-up dealer-is-small-blind rule, all-in-for-less-than-blind, `next_player`/`everyone_has_bet_once` round-completion arithmetic, phase progression, `new_hand` recursion. Considered clean (matches Go 1:1).
- Side pots (`showdown`): traced multi-level all-in splits, folded-player contribution absorption, uncalled-bet return, uneven-split remainder; proved the `while pot() > 0` loop always terminates (every folded player folded behind a strictly increasing current bet, so an active bet level always exists to absorb their contribution). Considered clean; the existing side-pot/uneven-split tests are good.
- Panic/unwrap reachability from crafted input: none found reachable — parser bounds raise amounts, all index/panic paths are guarded by invariants (see nits above).
- Serde views: `PubState`/`PlayerState` leak nothing private (hole cards only to owner; covered by tests).
- Bins: all four (`texas_holdem_2_cli/http/repl/fuzz.rs`) match the standard boilerplate pattern exactly, no deviations. Systemic duplication NOT reported per instructions.
- Documented Go quirks verified against the Go source and NOT reported: `can_raise` using `largest_raise` (lib.rs:298-310, genuine), `cards_by_suit` Spades sort skip (poker.rs:294-323, genuine), `pop_n` end-of-deck semantics (card.rs:95-109, genuine), fixed-width padding by `rank != 10` (card.rs:152-167, genuine), fold single-element loop flattening (lib.rs:214-221, genuine).

Findings: 2 minor, 4 nit. No critical or major findings. The crate is a careful, well-commented port; the only real defect is the raise-parser min bound divergence with its incorrect quirk comment, plus the undocumented 8-vs-9 player cap.
