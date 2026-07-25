# WP-25: modern-art-2 liveness and cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Eliminate the critical infinite busy-loop in `settle_auction` (d F34) and the round-4 soft-lock in `end_round` (d F35) with ONE shared invariant — "when play must pass to a new player, skip empty hands; if every hand is empty, the round is over" — plus round-4 regression tests. Also land the mechanical riders in the same crate: stale `State::Auction` on game end (d F41), the "$0 by auctioneer" render line (d F42), two RULES.md corrections (d F39, d F40), and three code nits (d F44, d F45, d F46).

**Architecture — how modern-art-2 works (read this before editing):**

- One crate, `rust/game/modern-art-2` (package name `modern-art-2`, confirmed from `Cargo.toml:2`; lib name `modern_art_2`): `src/lib.rs` (state machine + `Gamer` impl + inline `#[cfg(test)] mod test` at lib.rs:780), `src/card.rs` (cards: 5 artists x 5 auction types, 70 cards), `src/command.rs` (parsers), `src/render.rs` (markup), `tests/contract.rs` (standard contract harness). Test module name is `test`, NOT `tests`.
- Game flow: 4 rounds (`ROUNDS = 4`, `round` is 0-based). `start_round` (lib.rs:277) deals per `round_cards` (lib.rs:90): round-4 (index 3) deals **0 cards** for every player count; hands PERSIST across rounds (never cleared). `player_purchases` are cleared at round start and count toward `suit_cards_on_table` (lib.rs:113), which also counts `currently_auctioning`.
- Turn flow: in `State::PlayCard` only `current_player` may act, and the ONLY available command is `play <card>` (`command_parser`, command.rs:17-42: `can_pass` is false outside auctions). Playing a card removes it from hand, starts an auction, and — if that artist now has >= 5 cards on the table — calls `end_round` immediately (lib.rs:423-425; the 5th card is not sold). Otherwise the auction runs (Open/FixedPrice/Sealed/Double/OnceAround) and ends in `settle_auction` (lib.rs:429), which moves the cards to the winner's purchases, sets `State::PlayCard`, calls `next_player()`, then skips empty-handed players in a `while` loop (lib.rs:452-459).
- `end_round` (lib.rs:305): ranks artists, pushes a `values` map onto `value_board`, pays every player for their purchases, then either finishes the game (`round == ROUNDS - 1`: lib.rs:350-366, sets `finished = true`) or advances: `round += 1; next_player(); start_round()` (lib.rs:367-371) — with NO empty-hand skip on this path.
- `command()` (lib.rs:647) currently appends the `placings_log` finished-game epilogue ONLY in the `Play` (lib.rs:668-674) and `Add` (lib.rs:686-692) arms, because today those are the only commands that can finish a game (5th-card trigger). Task 1 creates finish paths through `Pass`/`Bid`/`Buy` (a settle can now end the round), so the epilogue is hoisted to cover all arms.
- Serialization: the whole `Game` is serde-persisted between requests; all fields are `pub`. **No fix in this package may change any serialized type or field.** Every change below is control flow, rendering, or docs.
- Deployment consequence of F34: each game crate runs as an HTTP service binary (via `brdgme_cmd`); a command that never returns hangs that request/worker while the log `Vec` grows one entry per loop iteration — unbounded memory growth until OOM. It is a plain infinite `while` loop, not recursion (no stack overflow; it allocates until killed).

**Tech Stack:** Rust 1.97.0 (edition 2024) workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `modern-art-2`. Tests: inline `mod test` in `src/lib.rs` plus the untouched `tests/contract.rs`. `Option::is_none_or` / `is_some_and` and let-chains are available on this toolchain.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p modern-art-2`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p modern-art-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- All 10 existing tests (`test_deck` ... `test_pub_state_hides_sealed_bids_and_money`) and `tests/contract.rs` MUST keep passing unmodified. If a fix changes their result, the fix is wrong — stop and re-check.
- Turn-order convention (docs/CODING.md): never `assert_ne!` on player index; the tests below use `assert_eq!` on the expected player plus log-content assertions, which is allowed.
- Tests must not assert on artist dollar values or payout amounts produced by `end_round`'s ranking/payment loops — those behaviors are owned by WP-26 (see Non-Goals) and asserting them here would lock in behavior pending adjudication.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).

**Non-Goals:**

- d F36 (cumulative payout for non-top-3 artists), d F37 (zero-card artists ranked and awarded $20/$10), d F43 (sealed/once-around ties favor the auctioneer) — rules-adjudication items owned by WP-26, blocked on decisions D-26/D-30/D-32. Do NOT touch `end_round`'s ranking loop (lib.rs:310-334), the payment loop (lib.rs:336-349), or `highest_bidder`'s tie behavior (lib.rs:190-202). **Known entanglement:** Task 1's fix means a round can now end with zero (or few) cards on the table, which exercises F37's zero-count ranking (all/most artists awarded from a 0-count table) and F36's payouts on that round. The liveness fix deliberately reuses `end_round` UNCHANGED — it decides *when* the round ends, never *how it scores*. That is why the tests assert only liveness facts (finished flag, current player, `value_board.len()`), never values.
- d F38 (`round_cards` `unreachable!()`/unchecked indexing on deserialized state) — owned by WP-09 (deserialized-state trust hardening, blocked on D-36). Do not touch `round_cards`' match/index structure.
- The cross-crate epilogue-dedup sweep (WP-08) does not include modern-art-2; Task 1's `command()` hoist is done here only because the new finish paths require it for correctness, not as part of that sweep. If WP-08 later lands a shared lib/game helper, this crate can adopt it then.
- The jaipur/sushi-go/lords-of-vegas findings in this batch — other packages.

**Snapshot drift:** None. `diff -r /home/beefsack/Development/brdgme-review-snapshot/rust/game/modern-art-2 /home/beefsack/Development/brdgme/rust/game/modern-art-2` is empty (verified 2026-07-25 against snapshot commit f8763a5). All line numbers below are live-file line numbers and match the findings' citations.

**Re-derivation of the busy-loop repro (F34) — verified against live source:**

The skip loop (lib.rs:452-459):

```rust
        self.state = State::PlayCard;
        self.next_player();
        while self.player_hands[self.current_player].is_empty() {
            logs.push(Log::public(vec![
                N::text("Skipping "),
                N::Player(self.current_player),
                N::text(" as they have no cards"),
            ]));
            self.next_player();
        }
```

has no all-hands-empty guard. The ONLY round-end trigger anywhere in the crate is `suit_cards_on_table(c.suit) >= 5` inside `add_card_to_auction` (lib.rs:423) — it fires on *playing* a card, never on settling. Legal-play reachability, step by step:

1. Rounds 1-3 each end on a 5th-card trigger, leaving unplayed cards in hands (hands persist; e.g. 3 players are dealt 10+6+6 = 22 cards each over rounds 1-3 and rounds end early).
2. Round 4 deals 0 cards (`round_cards` row `[.., 0]`), so hands entering round 4 are exactly those leftovers.
3. In `State::PlayCard` the current player's only legal command is `play <card>` — there is no pass, no decline. Players MUST play out their hands.
4. If the leftover cards jointly contain **at most 4 of any single artist** (common — e.g. players jointly enter round 4 holding 2 Lite Metal + 2 Yoko), the >= 5 trigger can never fire in round 4. Every card is played and auctioned; each auction settles normally.
5. When the LAST remaining card's auction settles, the seller's hand just emptied and everyone else's was already empty: `next_player()` lands on an empty hand, the loop pushes a "Skipping" log, advances, lands on another empty hand, forever. The `command()` call never returns; the HTTP request hangs and the log `Vec` grows without bound until OOM.

Minimal deterministic end-state (used by the Task 1 test): round 4, one player holds exactly 1 card, all other hands empty, purchases such that playing it does not reach 5-of-an-artist. Play it, settle the auction — loop.

The same loop can also strike in rounds 2-3: the trigger needs 5 cards of one artist *this round*, so if all hands empty out with every artist at <= 4 (possible when leftover-plus-dealt totals are <= 20 across 5 artists), the settle spins identically. The fix therefore ends *the current round* (which for rounds 1-3 deals the next round and continues) rather than special-casing round 4.

**Re-derivation of the soft-lock (F35):** `end_round`'s advance branch (lib.rs:367-371) is `self.round += 1; self.next_player(); logs.extend(self.start_round());` — no skip. Entering rounds 2-3 this is harmless (everyone is dealt > 0 cards), but entering round 4 (deals 0), if the player after the round-3-ender has an empty hand, the game lands in `State::PlayCard` with `whose_turn_players() == [that player]` and a `play` parser built over `Enum::exact` of an EMPTY hand (command.rs:44-48) — a parser that exists but can match no input, with no pass available. Nobody else may act. Permanent soft-lock. The one missing invariant is the same as F34's: the round-boundary path never skips empty hands and never detects that no hands remain.

**What the rules say (correct-behavior basis, in-repo):** the crate's own RULES.md already documents both halves of the invariant — line 78: "If a player has no cards in hand when it becomes their turn, they're skipped." and lines 83-84: a round "also ends naturally once all rounds' cards have been played out." The code implements the skip only on the settle path and the natural end nowhere. The fix implements exactly what RULES.md documents; no rules ambiguity and no RULES.md change is needed for F34/F35.

---

### Task 1: shared empty-hand invariant at both round boundaries (d F34 CRITICAL + d F35 MAJOR)

**Fix (re-derived, one design for both findings):** add one private helper that owns the invariant and call it from both sites.

```rust
    /// After play passes to a new current_player: skip players with no cards
    /// (RULES.md: "If a player has no cards in hand when it becomes their
    /// turn, they're skipped"). If EVERY hand is empty the round is over
    /// (RULES.md: a round "ends naturally once all rounds' cards have been
    /// played out") - without this check the skip loop would never terminate.
    fn advance_past_empty_hands(&mut self) -> Vec<Log> {
        if self.player_hands.iter().all(|h| h.is_empty()) {
            return self.end_round();
        }
        let mut logs = vec![];
        while self.player_hands[self.current_player].is_empty() {
            logs.push(Log::public(vec![
                N::text("Skipping "),
                N::Player(self.current_player),
                N::text(" as they have no cards"),
            ]));
            self.next_player();
        }
        logs
    }
```

- `settle_auction`: replace the raw `while` loop (lib.rs:452-459) with `logs.extend(self.advance_past_empty_hands());` (the `self.state = State::PlayCard; self.next_player();` lines above it stay).
- `end_round` advance branch (lib.rs:367-371): after `logs.extend(self.start_round());` add `logs.extend(self.advance_past_empty_hands());`.

Why this terminates: the `while` loop is now only entered when at least one hand is non-empty, so it runs at most `players - 1` iterations. The mutual recursion `advance_past_empty_hands -> end_round -> start_round -> advance_past_empty_hands` strictly increases `round` on every cycle and `end_round` returns without recursing once `round == ROUNDS - 1` (sets `finished`), so depth is bounded by `ROUNDS` (in practice 2: round 3 -> round 4 -> finish). The finding's own recommendation (bound the loop / break into `end_round` inside the settle loop only) is CORRECT but INSUFFICIENT alone — it does not fix F35's round-transition path; the shared helper covers both with the single invariant, which is why they are fixed together.

Why calling `end_round` (not a bespoke "finish") is right: for rounds 1-3 an all-empty settle means that round's cards are exhausted — the round must score and the next round must deal (RULES.md natural end); for round 4 `end_round` finishes the game. One call handles both. Scoring behavior inside `end_round` is untouched (see Non-Goals entanglement note).

**`command()` epilogue hoist (required by this fix):** settles can now finish the game, and settles are reached from `Pass`, `Bid`, and `Buy` commands, whose arms lack the `placings_log` epilogue that `Play`/`Add` have. Restructure `command()` (lib.rs:662-736) so every arm produces `(remaining, logs)` and ONE shared post-step appends the epilogue:

```rust
        let (remaining, mut logs) = match output {
            Ok(ParseOutput { remaining, value: Command::Play(c), .. }) => {
                (remaining, self.play_card(player, c)?)
            }
            Ok(ParseOutput { remaining, value: Command::Add(c), .. }) => {
                (remaining, self.add_card(player, c)?)
            }
            Ok(ParseOutput { remaining, value: Command::Bid(amount), .. }) => {
                (remaining, self.bid(player, amount)?)
            }
            Ok(ParseOutput { remaining, value: Command::Buy, .. }) => (remaining, self.buy(player)?),
            Ok(ParseOutput { remaining, value: Command::Pass, .. }) => {
                (remaining, self.pass(player)?)
            }
            Ok(ParseOutput { remaining, value: Command::Price(amount), .. }) => {
                (remaining, self.set_price(player, amount)?)
            }
            Err(e) => return Err(e),
        };
        if self.is_finished() {
            let scores: Vec<(usize, i32)> = (0..self.players)
                .map(|p| (p, self.player_money[p]))
                .collect();
            logs.push(placings_log(&self.placings(), Some(&scores)));
        }
        Ok(CommandResponse {
            logs,
            can_undo: false,
            remaining_input: remaining.to_string(),
        })
```

This is behavior-identical for the existing finish paths (the epilogue only ever fires on the finishing command — once `finished` is true, `command_parser` returns `None` and every later command errors before reaching the match, so no double-logging is possible).

**Edge cases (all traced against live source):**

- 3/4/5 players: nothing player-count-specific; the skip bound is `players - 1`.
- All hands empty after a settle in rounds 1-3 (legal when all artists stay <= 4 that round): `end_round` scores the round, next round DEALS cards (rounds 2-3 deal > 0 for every player count), so `advance_past_empty_hands` after `start_round` no-ops and play continues. Not a game end.
- All hands empty entering round 4 (round 3's 5th-card trigger consumed the last card in play): `end_round` -> `start_round` (deals 0, logs "Start of round 4") -> `advance_past_empty_hands` -> all empty -> `end_round` -> finished. Round 4 starts and immediately ends — scored by the existing (WP-26-pending) logic on a 0-card table.
- Next player empty entering round 4 but someone still has cards (F35's exact case): skipped with the same "Skipping" log the settle path uses; lands on the first player with cards.
- Only one player has cards after a settle: loop skips up to `players - 1` seats and stops on them (possibly the auction winner themselves — they simply act again, matching the pre-existing settle-skip semantics).
- Double auction with an added card: `add_card_to_auction` set `current_player` to the ADDER, so the settle advances from the adder — unchanged; the helper is downstream of that.
- 5th-card trigger: `end_round` is called from `add_card_to_auction` BEFORE any settle (the card is unsold); `settle_auction` is not on that path, so the trigger path only gains the post-`start_round` skip.
- Deck exhaustion mid-deal (`start_round` drains `num_cards.min(deck.len())` per player): impossible with the standard 70-card deck and the deal table, but if it ever produced an empty dealt hand the post-deal skip now handles it instead of soft-locking.
- Finished game: `end_round`'s finished branch returns before any advance; `whose_turn_players` short-circuits on `finished`; `command_parser` returns `None`.
- Corrupt/deserialized state where `current_player >= players` or hands vec is short: out of scope (WP-09 / d F38 class); the helper indexes exactly as the current code does.

**Files:**
- Modify: `rust/game/modern-art-2/src/lib.rs` (`settle_auction` lines 452-459, `end_round` lines 367-371, new helper in `impl Game`, `command()` lines 662-736)
- Test: same file, inline `mod test`

**Steps:**

- [ ] Write the failing tests. Add to `mod test` in `rust/game/modern-art-2/src/lib.rs` (a `log_plain` helper is added first — mirrors alhambra-1's, lib.rs:1033 there):

```rust
    fn log_plain(log: &Log) -> String {
        brdgme_markup::plain(&brdgme_markup::transform(&log.content, &[]))
    }

    #[test]
    fn all_hands_empty_after_settle_ends_the_game() {
        // d F34: settling the last auction of round 4 with every hand empty
        // must end the round (and the game), not busy-loop forever. The
        // settling command runs on a worker thread with a short timeout so a
        // regression FAILS in ~2s instead of hanging CI while allocating
        // unbounded "Skipping" logs.
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        let p = players(3);
        let mut g = Game::start(3, 1).unwrap().0;
        // Round 4: no cards are dealt; hands are round-3 leftovers. MICK
        // holds the last card in the game; playing it cannot reach 5 of an
        // artist, so the auction must run and settle.
        g.round = 3;
        g.state = State::PlayCard;
        g.current_player = MICK;
        g.player_hands = vec![
            vec![Card {
                suit: Suit::LiteMetal,
                rank: Rank::Open,
            }],
            vec![],
            vec![],
        ];
        g.player_purchases = vec![vec![]; 3];
        g.command(MICK, "play lmop", &p).unwrap();
        g.command(STEVE, "pass", &p).unwrap();
        // BJ's pass settles the auction (auctioneer wins at $0) with every
        // hand now empty - the pre-fix skip loop never terminates here.
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = g.command(BJ, "pass", &p);
            let _ = tx.send((g, result));
        });
        let (g, result) = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("settle with all hands empty must terminate (d F34 busy-loop)");
        let resp = result.unwrap();
        assert!(g.is_finished(), "round 4 with no cards left must end the game");
        assert_eq!(1, g.value_board.len(), "the round must have been scored");
        assert!(g.whose_turn_players().is_empty());
        // The finishing command came through the Pass arm - the hoisted
        // epilogue must still announce the result.
        assert!(
            resp.logs.iter().any(|l| {
                let t = log_plain(l);
                t.contains("wins!") || t.contains("tie!")
            }),
            "finishing via pass must emit the placings log"
        );
    }

    #[test]
    fn round_four_skips_empty_handed_starter() {
        // d F35: round 3 ends on the 5th Lite Metal; STEVE (next in turn
        // order) has no cards and round 4 deals none - STEVE must be skipped,
        // not left as a soft-locked current player.
        let p = players(3);
        let mut g = Game::start(3, 1).unwrap().0;
        g.round = 2;
        g.state = State::PlayCard;
        g.current_player = MICK;
        let lm_open = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Open,
        };
        let yo_open = Card {
            suit: Suit::Yoko,
            rank: Rank::Open,
        };
        g.player_purchases = vec![vec![lm_open, lm_open], vec![lm_open, lm_open], vec![]];
        g.player_hands = vec![vec![lm_open], vec![], vec![yo_open]];
        let resp = g.command(MICK, "play lmop", &p).unwrap();
        assert_eq!(3, g.round, "round 4 must have started");
        assert!(!g.is_finished());
        assert_eq!(State::PlayCard, g.state);
        assert_eq!(
            BJ, g.current_player,
            "empty-handed STEVE must be skipped when round 4 starts (d F35)"
        );
        assert!(
            resp.logs.iter().any(|l| log_plain(l).contains("Skipping")),
            "the skip must be logged"
        );
    }

    #[test]
    fn round_four_with_no_cards_anywhere_ends_immediately() {
        // d F34+F35 boundary: the round-3 trigger consumed the last card in
        // play, so round 4 starts with every hand empty and must end at once.
        let p = players(3);
        let mut g = Game::start(3, 1).unwrap().0;
        g.round = 2;
        g.state = State::PlayCard;
        g.current_player = MICK;
        let lm_open = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Open,
        };
        g.player_purchases = vec![vec![lm_open, lm_open], vec![lm_open, lm_open], vec![]];
        g.player_hands = vec![vec![lm_open], vec![], vec![]];
        g.command(MICK, "play lmop", &p).unwrap();
        assert!(
            g.is_finished(),
            "round 4 with no cards in any hand must end the game immediately"
        );
        assert_eq!(
            2,
            g.value_board.len(),
            "both round 3 and round 4 must have been scored"
        );
        assert!(g.whose_turn_players().is_empty());
    }

    #[test]
    fn settle_skips_empty_hands_when_cards_remain() {
        // Lock-in: the pre-existing skip behavior (some hands empty, at least
        // one not) is unchanged by the fix.
        let p = players(3);
        let mut g = Game::start(3, 1).unwrap().0;
        g.round = 3;
        g.state = State::PlayCard;
        g.current_player = MICK;
        g.player_hands = vec![
            vec![Card {
                suit: Suit::LiteMetal,
                rank: Rank::Open,
            }],
            vec![],
            vec![Card {
                suit: Suit::Yoko,
                rank: Rank::Open,
            }],
        ];
        g.player_purchases = vec![vec![]; 3];
        g.command(MICK, "play lmop", &p).unwrap();
        g.command(STEVE, "pass", &p).unwrap();
        g.command(BJ, "pass", &p).unwrap();
        assert!(!g.is_finished());
        assert_eq!(State::PlayCard, g.state);
        assert_eq!(
            BJ, g.current_player,
            "STEVE (no cards) skipped; BJ still holds a card"
        );
    }
```

  Trace notes for the implementer (do not skip): in the F34 test, MICK's play puts 1 Lite Metal on the table (< 5, auction runs); Open-auction `whose_turn` is STEVE+BJ (MICK is the $0 sentinel highest bidder); after both pass, `settle_auction(MICK, 0)` runs inside BJ's `pass` command. In the F35/boundary tests, the played card makes `suit_cards_on_table(LiteMetal) == 5` (4 in purchases + 1 auctioning), so `end_round` fires from `add_card_to_auction` and the card is never sold. `value_board.len()` starts at 0 because `round` was jumped manually. Do NOT assert money or artist values (WP-26).

- [ ] Run the fast-failing tests first: `cargo test -p modern-art-2 round_four` — expected: `round_four_skips_empty_handed_starter` FAILS on `assert_eq!(BJ, g.current_player)` (left `2`, right `1` — STEVE is stuck as current player) and `round_four_with_no_cards_anywhere_ends_immediately` FAILS on `assert!(g.is_finished())` (game soft-locked, not finished). Then run the busy-loop test ALONE: `cargo test -p modern-art-2 all_hands_empty_after_settle` — expected: FAILS after ~2s on the `recv_timeout` expect ("settle with all hands empty must terminate"). CAUTION: during those ~2s the leaked worker thread allocates log entries rapidly; run this test by itself for the red run and do not leave it looping. `settle_skips_empty_hands_when_cards_remain` PASSES already (it locks in current behavior).
- [ ] Implement, in `rust/game/modern-art-2/src/lib.rs`:
  1. Add `advance_past_empty_hands` (code above) to `impl Game`, next to `settle_auction`.
  2. In `settle_auction`, replace the `while` loop (lines 452-459) with `logs.extend(self.advance_past_empty_hands());`.
  3. In `end_round`'s advance branch, after `logs.extend(self.start_round());` add `logs.extend(self.advance_past_empty_hands());`.
  4. Restructure `command()` per the hoist code above (delete the per-arm epilogue duplicates in the Play/Add arms).
- [ ] Run: `cargo test -p modern-art-2` — all 4 new tests PASS, all 10 existing tests + contract test PASS. In particular `test_open_auction`/`test_sealed_auction`/`test_once_around_auction`/`test_fixed_price_auction` confirm normal settles are unchanged (their hands are never all-empty) and `test_end_of_round` confirms the 5th-card finish path still emits the same results through the hoisted epilogue.
- [ ] `cargo clippy -p modern-art-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/modern-art-2/src/lib.rs` ; message: `fix(modern-art-2): end the round when all hands are empty at any boundary (d F34 d F35, WP-25)`

---

### Task 2: reset state when the game ends mid-auction (d F41, minor)

**Problem (restated):** when the game ends via the 5th-card trigger in round 4, the 5th card was just pushed and `self.state = State::Auction` set (lib.rs:422) before `end_round` runs; the finished branch (lib.rs:350-366) sets `finished = true` but never resets `state`. `whose_turn_players`/`status` short-circuit on `finished` so there is no deadlock, but `pub_state().is_auction` stays `true` and the final screen renders "<player> is auctioning " with an EMPTY card list (render.rs:57-60; `currently_auctioning` was cleared at lib.rs:309) plus a bogus "Current bid: $0 by <auctioneer>" line (verification-confirmed extra symptom; the non-Sealed `current_bid` path at lib.rs:628-632 still fires). Display-only defect.

**Fix (re-derived):** set `self.state = State::PlayCard;` at the TOP of `end_round`, next to the `currently_auctioning` clear (lib.rs:309). This covers the finished branch AND is a harmless no-op on the advance branch (`start_round` sets it again) and on Task 1's settle path (already `PlayCard`). One line, both branches, no per-branch duplication. With `is_auction()` false, `pub_state` yields `is_auction: false`, `auction_type: None`, `current_bid: None` — the renderer's whole auction block is skipped, fixing both bogus lines at once.

**Edge cases:** mid-game 5th-card trigger (rounds 1-3): behavior identical (state was reset by `start_round` anyway, now merely earlier); settle-then-round-end path from Task 1: state already `PlayCard`; finished game re-rendered from persisted state: the persisted `state` is now `PlayCard`, so old-broken saves that already finished with `Auction` stuck are NOT retroactively fixed (acceptable — display-only, and `State` is just an enum field; no migration warranted for a cosmetic line on already-finished games).

**Files:**
- Modify: `rust/game/modern-art-2/src/lib.rs` (`end_round`, one line near lib.rs:309)
- Test: same file, inline `mod test`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn game_end_via_fifth_card_leaves_no_stale_auction() {
        // d F41: the 5th card in round 4 ends the game mid-"auction"; the
        // final public state must not claim an auction is running.
        let p = players(4);
        let mut g = mock_game();
        let lm_open = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Open,
        };
        g.round = 3;
        g.player_purchases[MICK] = vec![lm_open, lm_open];
        g.player_purchases[STEVE] = vec![lm_open];
        g.player_purchases[BJ] = vec![lm_open];
        g.player_hands[MICK].push(lm_open);
        g.command(MICK, "play lmop", &p).unwrap();
        assert!(g.is_finished());
        let ps = g.pub_state();
        assert!(!ps.is_auction, "finished game must not report an auction");
        assert!(ps.auctioning.is_empty());
        assert_eq!(None, ps.auction_type);
        assert_eq!(None, ps.current_bid, "no bogus $0 bid on the final screen");
    }
```

  (Setup mirrors the existing `test_end_of_round` final-round block, which already proves this play sequence finishes the game.)

- [ ] Run: `cargo test -p modern-art-2 leaves_no_stale_auction`. Expected: FAIL on `assert!(!ps.is_auction)` — pre-fix the state is still `Auction`.
- [ ] Implement: in `end_round`, directly after `self.currently_auctioning = vec![];` (lib.rs:309) add:

```rust
        // The round can end mid-auction (5th-card trigger): the auction is
        // void, so leave PlayCard state - otherwise a finished game renders a
        // phantom "is auctioning" screen.
        self.state = State::PlayCard;
```

- [ ] Run: `cargo test -p modern-art-2` — new test PASSES, full suite PASSES (`test_double_auction_ends_round` and `test_end_of_round` exercise the mid-game trigger path).
- [ ] `cargo clippy -p modern-art-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/modern-art-2/src/lib.rs` ; message: `fix(modern-art-2): reset auction state when a round ends (d F41, WP-25)`

---

### Task 3: suppress the "$0 by auctioneer" bid line (d F42, nit)

**Problem (restated):** for any non-Sealed auction with no bids yet, `pub_state` sets `current_bid = Some(self.highest_bidder())` (lib.rs:628-632) and `highest_bidder`'s `-1` sentinel start (lib.rs:190-202) returns `(auctioneer, 0)`, so the renderer (render.rs:62-71) prints "Current bid: $0 by <auctioneer>" before anyone has bid.

**Fix (re-derived — renderer side, NOT pub_state):** gate the render on `bid > 0`. A real bid can never be 0 (`min_bid()` >= 1 always: sealed floor is 1, otherwise `highest + 1` with `highest >= 0`; `bid()` rejects `amount <= highest_bid`; `set_price` rejects `price <= 0`), so `bid > 0` is exactly "someone has actually bid / a price is set". The finding's alternative — returning `None` from `pub_state` until a real bid — is REJECTED here: the `(auctioneer, $0)` sentinel is documented API surface (doc comment lib.rs:70 and `DATA_DOCS.md:12` both state "When nobody has bid yet this is the auctioneer at $0"), and changing it would alter the serialized `PubState` contract for a nit. The render-side gate fixes the player-visible symptom and keeps the data contract intact.

**Edge cases:** FixedPrice after `price N` is set: `bids[auctioneer] = N >= 1`, so the line still renders (unchanged, correct — it shows the asking price); Sealed: already excluded by the existing `!= Some(Rank::Sealed)` condition; finished game: auction block already skipped after Task 2; Double before any bid: `(auctioneer, 0)` — now suppressed, same as Open.

**Files:**
- Modify: `rust/game/modern-art-2/src/render.rs` (line 62-63 condition)
- Test: `rust/game/modern-art-2/src/lib.rs`, inline `mod test`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn no_current_bid_line_before_any_bid() {
        // d F42: an open auction with no bids must not render
        // "Current bid: $0 by <auctioneer>".
        use brdgme_game::Renderer;
        let p = players(4);
        let mut g = mock_game();
        g.current_player = BJ;
        g.player_hands[BJ].push(Card {
            suit: Suit::LiteMetal,
            rank: Rank::Open,
        });
        g.command(BJ, "play lmop", &p).unwrap();
        let no_bids =
            brdgme_markup::plain(&brdgme_markup::transform(&g.pub_state().render(), &[]));
        assert!(
            !no_bids.contains("Current bid"),
            "no bid line before any bid, got:\n{}",
            no_bids
        );
        g.command(STEVE, "bid 10", &p).unwrap();
        let with_bid =
            brdgme_markup::plain(&brdgme_markup::transform(&g.pub_state().render(), &[]));
        assert!(
            with_bid.contains("Current bid: $10"),
            "real bids must still render, got:\n{}",
            with_bid
        );
    }
```

  (If `Renderer` is not in scope via `brdgme_game`, the import is `use brdgme_game::Renderer;` as shown — check `render.rs:2` for the trait path.)

- [ ] Run: `cargo test -p modern-art-2 no_current_bid_line`. Expected: FAIL on the first assert — the pre-fix render contains "Current bid: $0".
- [ ] Implement: in `render.rs`, extend the let-chain condition (lines 62-63) to:

```rust
        if pub_state.auction_type != Some(Rank::Sealed)
            && let Some((bidder, bid)) = pub_state.current_bid
            && bid > 0
        {
```

- [ ] Run: `cargo test -p modern-art-2` — new test PASSES, full suite PASSES.
- [ ] `cargo clippy -p modern-art-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/modern-art-2/src/render.rs rust/game/modern-art-2/src/lib.rs` ; message: `fix(modern-art-2): hide the bid line until a real bid exists (d F42, WP-25)`

---

### Task 4: RULES.md corrections (d F39 + d F40, minor doc-only)

**Problem (restated):**

- d F39 — `RULES.md:73-76` says the auction winner "adds the card(s) to their purchases ... and it becomes their turn next." Wrong: the implementation (and official rules and the Go port, per verification) passes the turn clockwise from the SELLER — `add_card_to_auction` sets `current_player` to whoever played/added the card (lib.rs:414) and `settle_auction` calls `next_player()` from there (lib.rs:451). Nuance to document precisely: for a Double auction with an added second card, `current_player` is the ADDER, so the turn passes to the adder's left (live behavior, confirmed by `test_double_auction`: ELVA plays, STEVE adds, MICK wins, BJ — STEVE's left — is next).
- d F40 — `RULES.md:63` says "Double - Works like Open, Fixed Price, or Sealed depending on the second card added", but `add_card` (lib.rs:385-402) only rejects a second Double, so a Once Around card is also a valid add (the auction then runs as Once Around). The doc list is incomplete; the code matches official rules.

**Fix:** doc edits only. No code, no tests.

**Files:**
- Modify: `rust/game/modern-art-2/RULES.md` (lines 63, 73-76)

**Steps:**

- [ ] Line 63: change "Works like Open, Fixed Price, or Sealed depending on the second card added" to "Works like Open, Fixed Price, Sealed, or Once Around depending on the second card added".
- [ ] Lines 73-76: replace

  > Whoever wins the auction pays their bid: to the auctioneer if someone else wins, or to the bank if the auctioneer wins their own auction. The winner adds the card(s) to their purchases (face-up, public knowledge) and it becomes their turn next.

  with

  > Whoever wins the auction pays their bid: to the auctioneer if someone else wins, or to the bank if the auctioneer wins their own auction. The winner adds the card(s) to their purchases (face-up, public knowledge). The next turn then passes to the player on the auctioneer's left (for a Double auction where a second card was added, to the left of the player who added it).

- [ ] Run: `cargo test -p modern-art-2` — full suite PASSES (`rules()` is `include_str!`; a build re-embed is the only effect).
- [ ] `cargo fmt --all -- --check` clean (no Rust touched; runs as part of the task convention).
- [ ] Commit: `git add rust/game/modern-art-2/RULES.md` ; message: `docs(modern-art-2): correct next-turn and Double-auction rules text (d F39 d F40, WP-25)`

---

### Task 5: code nits — throwaway vec, guarded unwrap, dead import (d F44 + d F45 + d F46, nits)

**Problem (restated):**

- d F44 — `can_add` (lib.rs:260): `!self.player_hands.get(player).unwrap_or(&vec![]).is_empty()` heap-allocates an empty `Vec` on every call purely as a fallback.
- d F45 — `whose_turn_players` Open arm (lib.rs:152): `p != highest_bidder && (bid.is_none() || *bid.unwrap() > 0)` — safe via short-circuit but uses `.unwrap()` in a runtime path (repo rules forbid it). Note the verification correction: the original finding mis-quoted the expression (dropped the `> 0`); the live expression is as shown here and the replacement below preserves it exactly.
- d F46 — `use std::default::Default;` (lib.rs:2) is redundant on edition 2024 (prelude).

**Fix:** three mechanical rewrites, behavior-identical, covered by the existing suite (every auction test exercises `can_add`/`whose_turn_players`). No new tests — no observable behavior to assert.

**Files:**
- Modify: `rust/game/modern-art-2/src/lib.rs` (lines 2, 152, 260)

**Steps:**

- [ ] lib.rs:260: change the last condition of `can_add` to `&& self.player_hands.get(player).is_some_and(|h| !h.is_empty())`.
- [ ] lib.rs:152: change the filter body to `p != highest_bidder && bid.is_none_or(|&b| b > 0)` (`bid` is `Option<&i32>`, so destructure the reference in the closure).
- [ ] lib.rs:2: delete `use std::default::Default;`.
- [ ] Run: `cargo test -p modern-art-2` — full suite PASSES (behavioral lock: `test_open_auction` walks the Open-arm filter through bid/pass states; `test_double_auction` walks `can_add`).
- [ ] `cargo clippy -p modern-art-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/modern-art-2/src/lib.rs` ; message: `refactor(modern-art-2): drop throwaway vec, unwrap, dead import (d F44 d F45 d F46, WP-25)`

---

## Findings disposition

| Finding | Severity | Disposition |
|---|---|---|
| d F34 settle busy-loop, all hands empty | critical | Fixed (Task 1): shared `advance_past_empty_hands` helper — skip loop only entered when some hand is non-empty; all-empty ends the round via existing `end_round`. Repro re-derived: legal, forced play in round 4 (0 cards dealt, hands persist, no 5th-of-artist trigger possible with <= 4 per artist among leftovers; players cannot decline to play). Consequence confirmed from source: infinite `while` (not recursion) pushing one log per iteration — request hang + unbounded allocation. Timeout-bounded regression test fails in ~2s instead of hanging CI. Finding's bound-the-loop recommendation was correct but insufficient alone (misses F35's path). |
| d F35 round-4 empty-handed starter soft-lock | major | Fixed (Task 1, same invariant): `end_round`'s advance branch now runs the shared skip/all-empty helper after `start_round`. Covers next-player-empty (skip to first player with cards) and all-empty-entering-round-4 (round 4 starts and immediately ends). Correct rule confirmed in-repo: RULES.md:78 (skip) and RULES.md:83-84 (natural round end) already document exactly this behavior — no rules ambiguity. |
| d F39 RULES.md "winner takes next turn" | minor | Fixed (Task 4): reworded to clockwise-from-auctioneer, including the re-derived Double-auction nuance (turn passes from the ADDER of the second card — lib.rs:414 + 451, confirmed by `test_double_auction`). |
| d F40 RULES.md Double omits Once Around | minor | Fixed (Task 4): Once Around added to the Double list (`add_card` only rejects a second Double). |
| d F41 stale State::Auction on game end | minor | Fixed (Task 2): `state = PlayCard` set at the top of `end_round` next to the `currently_auctioning` clear — covers the finished branch, no-op elsewhere. Also removes the verification-noted bonus symptom (bogus "$0" line on the final screen). Already-persisted finished games keep the stale flag (display-only; no migration warranted). |
| d F42 "Current bid: $0" before any bid | nit | Fixed (Task 3): render-side `bid > 0` gate. Overturned half of the finding's recommendation: pub_state must NOT change — the `(auctioneer, $0)` sentinel is documented API (lib.rs:70 doc comment, DATA_DOCS.md:12); `bid > 0` is provably equivalent to "a real bid/price exists" (min_bid >= 1 on every path). |
| d F44 `can_add` throwaway `vec![]` | nit | Fixed (Task 5): `is_some_and(|h| !h.is_empty())`. |
| d F45 guarded `bid.unwrap()` | nit | Fixed (Task 5): `bid.is_none_or(|&b| b > 0)` — preserves the live expression including the `> 0` the original finding mis-quoted (verification-adjusted detail). |
| d F46 redundant `use std::default::Default` | nit | Fixed (Task 5): deleted (edition 2024 prelude). |
