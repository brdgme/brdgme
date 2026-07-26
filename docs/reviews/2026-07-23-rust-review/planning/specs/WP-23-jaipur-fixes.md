# WP-23: jaipur-2 fixes

> **CITATION WARNING - line numbers in this spec are approximate and unverified.**
> Corpus-wide they measured **33-46% wrong**, and two "delete lines A-B" ranges
> would have destroyed live code. **Navigate by the named function, type or
> symbol** - never by line number alone. If the code at a cited location does not
> match this spec's description, **STOP and report**; do not improvise a fix or
> guess at the intended target.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Award the bonus token for 6- and 7-card sales, which the crate currently drops on the floor (d F14, major); stop mixed-type sell input (`sell dia gold lea`) from silently executing an unintended same-type sale (d F18, minor) and remove the silent `Good::Diamond` fallback that hides parser regressions (d F20, nit); replace the one-line RULES.md stub with real rules so `Gamer::rules()` serves something (d F17, minor); delete the dead `parsers.is_empty()` branch in `command_parser` (d F19, nit); and stop the renderer claiming a fixed number of remaining rounds in a first-to-2 match (d F22, nit).

**Architecture — how jaipur-2 works (read this before editing):**

- One crate, `rust/game/jaipur-2` (package name `jaipur-2`, `Cargo.toml:2`; lib name `jaipur_2`, edition 2024). Two players only (`NUM_PLAYERS = 2`, lib.rs:20; `player_counts()` returns `vec![2]`, lib.rs:808-810).
- `src/lib.rs` (1553 lines): the `Good` enum and its data tables (`card_count` lib.rs:97-107, `min_sale` lib.rs:109-115, `token_values` lib.rs:117-127), the free functions `bonus_values` (lib.rs:136-143), `bonus_sizes` (lib.rs:145-147) and `initial_deck` (lib.rs:149-157), the serde-persisted `Game` struct (lib.rs:159-174), `PubState` (lib.rs:176-196), `PlayerState` (lib.rs:198-206), the round engine (`start_round` lib.rs:213-250, `replenish_market` lib.rs:252-274, `receive_cards` lib.rs:276-311, `take_camels` lib.rs:317-348, `take_goods` lib.rs:350-479, `sell` lib.rs:485-577, `end_round` lib.rs:579-642), the `Gamer` impl (lib.rs:663-831) and a large inline `#[cfg(test)] mod tests` (lib.rs:833-1553, **60 tests**).
- `src/command.rs` (91 lines, private `mod command;` at lib.rs:16 with `pub use command::Command;` at lib.rs:18): the `Command` enum (`Take { take, give }`, `Sell { good, quantity }`, command.rs:5-9), `Game::command_parser` (command.rs:11-25), `good_parser`/`trade_good_parser` (command.rs:27-33), `take_parser` (command.rs:35-58), `sell_parser` (command.rs:60-91). `Command` is **not** serialized — it derives only `Debug, PartialEq, Clone` (command.rs:5).
- `src/render.rs` (268 lines): `render_good`, `render_goods_list`, `render_goods_items` (the last is re-imported by lib.rs:14 for log text), `camel_display` (render.rs:40-42), `render_token_table` (render.rs:62-130), `render_bonus_table` (render.rs:134-159 — the third column is labelled **"5 or more"**, render.rs:153), `common_rows` (render.rs:173-200), `you_have_rows`, `opponent_rows`, and the two `Renderer` impls (render.rs:249-268).
- Game flow: a match is best-of-three, first to 2 round wins (`is_finished()` lib.rs:648-650). `start_round` (lib.rs:213-250) shuffles a fresh 52-card deck, seeds the market with **3 camels placed directly rather than drawn** (lib.rs:223), replenishes the market to 5 from the deck, deals 5 cards to each player, then rebuilds the `goods` token piles and the three shuffled `bonuses` piles keyed 3/4/5. A turn is either a take (single good / multi-good exchange / all camels) or a sell. A round ends when 3 goods piles are exhausted (checked in `sell`, lib.rs:567-575) or when the deck cannot refill the market to 5 (`replenish_market`, lib.rs:257-260).
- Serialization: `Game`/`PubState`/`PlayerState` round-trip through the DB as serde JSON (`state_round_trips_through_json`, lib.rs:946-957). **No task in this package changes any serialized type, field name, or field shape.** F14 changes only which map key `sell()` looks up; F18/F19/F20 touch the transient parser; F22 touches the renderer; F17 is a markdown file. `Command` is transient, and the `CommandSpec` produced by `command_spec()` (lib.rs:798-800) is preserved byte-identically by Task 2 (see its `to_spec` delegation).
- Bins (`src/bin/jaipur_2_{cli,fuzz,http,repl}.rs`) are the four standard boilerplate binaries over `jaipur_2::Game`. `tests/contract.rs` is the standard `assert_gamer_contract::<Game>()` harness; note it asserts `rules must not be empty` (`rust/lib/cmd/src/test_support.rs:32-37`) — the current stub passes that check, and Task 5 keeps it passing.

**Tech Stack:** Rust 1.97.0 (edition 2024) workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `jaipur-2`. Let-chains (already used at lib.rs:521-523 and command.rs) and let-else are available on this toolchain.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p jaipur-2`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p jaipur-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- Baseline verified 2026-07-25: `cargo test -p jaipur-2` is **61 green** (60 inline unit tests + `game_contract`). All 61 MUST keep passing unmodified. Checked individually against every fix below; the ones nearest the blast radius are `sell_succeeds_and_collects_tokens` (lib.rs:1063-1073, a 2-gold sale — below the bonus threshold, unaffected by Task 1), `sell_with_bonus_includes_private_bonus_log` (lib.rs:1210-1221, a 3-leather sale — key 3, unchanged by the clamp), `sell_parser_parses_quantity_prefix` (lib.rs:1018-1030, `sell 2 gold` — goes through the `Int` sub-parser, untouched by Task 2), `command_preserves_remaining_input` (lib.rs:1544-1552, `sell 2 gold and then`), and the three render tests (lib.rs:1223-1237, 1271-1294) which assert only non-emptiness, the literal `"You have"`, and token-table ordering.
- Line numbers cited are LIVE-file numbers as of the drift check below. Task 1 shifts lib.rs numbering below ~line 520 by a few lines and Task 2 shifts command.rs numbering below ~line 33 — later tasks locate by symbol name where noted.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).

**Non-Goals:**

- **d F13 (8 camels / 52 cards vs official 11 / 55) was REJECTED by verification.** `Good::card_count` (lib.rs:97-107) is the *deck* composition, not the total component list: `start_round` places 3 camels into the market directly (lib.rs:223) instead of drawing them, so 8 deck camels + 3 market camels = **11 camels in play**, and the deck lands at exactly 40 after setup (verified live: `start_deck_is_40`, lib.rs:912-916, passes). The finding's recommended fix (`Good::Camel => 11`) would create **14** camels — a real bug. **Do NOT change `card_count`, `deck_has_52_cards` (lib.rs:837-840), or `start_deck_is_40`.** Task 5's RULES.md explicitly documents the 8/3 split so no future reader repeats this mistake.
- d F15 (next-round starting player is not the round loser) — owned by **WP-26** (batch-d rules adjudication, BLOCKED-ON-DECISION). Do NOT touch `end_round` (lib.rs:579-642), `next_player` (lib.rs:644-646), or the `emptied >= 3` branch (lib.rs:571-575), and do NOT write a starting-player rule into RULES.md.
- d F16 (camel token counted as a bonus token for the first tie-break) — owned by **WP-26**. Do NOT touch `self.bonus_tokens[cw] += 1` (lib.rs:598) or the tie-break chain (lib.rs:613-627). Task 5's RULES.md deliberately states the tie-break order *without* saying whether the camel token counts as a bonus token, so it stays correct whichever way WP-26 decides.
- d F23 (opponent camel display hides the count `PubState.camels` already exposes) — owned by **WP-26**. Do NOT touch `camel_display` (render.rs:40-42) or `opponent_rows` (render.rs:224-247), even though Task 4 edits the same file.
- d F21 (placings-log block duplicated between the Take and Sell arms, lib.rs:754-764 and 777-787) — **not in this package**: work-packages.md:81 assigns `d F21` to the cross-crate epilogue/placings dedup package. Leave `command()` (lib.rs:725-796) alone.
- Changing the deck-exhaustion round-end threshold in `replenish_market` (lib.rs:257-260, "round ends when the deck cannot refill to 5") — not a filed finding; verification recorded round-end triggers as clean.
- Any change to `Good`, `Game`, `PubState` or `PlayerState` field sets, names, or types. Nothing in this package needs one.

**Snapshot drift:** None. `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/jaipur-2 /home/beefsack/Development/brdgme/rust/game/jaipur-2` produced no output and exited 0 (verified 2026-07-25). All line numbers below are live-repo numbers and match the findings' citations.

**Re-derivation notes (verified against live source, and empirically where noted):**

- **F14 (no bonus for 6/7-card sales) — CONFIRMED, reproduced live.** `sell()` looks the bonus pile up by the raw quantity: `self.bonuses.get_mut(&quantity)` (lib.rs:521). The `bonuses` map is built in `start_round` with exactly the keys from `bonus_sizes()` = `MIN_TRADE_BONUS..=MAX_TRADE_BONUS` = 3..=5 (lib.rs:242-247, 145-147). `HAND_SIZE = 7` (lib.rs:21), and the deck holds 8 cloth, 8 spice and 10 leather (lib.rs:102-104), so 6- and 7-card sales of a common good are ordinary play; even the rare goods reach 6 (6 diamonds in the deck). Reproduced with a throwaway integration test on the live crate: selling 6 leather from a 6-leather hand left the 5-pile at **5 tokens (unchanged)**, `bonus_tokens` at `[0, 0]`, and logged `"sold 6 leathers for 12 points"` with no bonus. The crate contradicts itself twice: `render_bonus_table` labels the third column **"5 or more"** (render.rs:153) and `DATA_DOCS.md:13` says bonus tokens are "awarded when selling 3+ of a good". The finding's recommendation (clamp the key) is correct; the fix uses the existing `MAX_TRADE_BONUS` constant instead of a literal `5`. Quantities of 1 and 2 keep getting nothing: `1.min(5) == 1` and the map has no key 1.
- **F18 (mixed-type sell silently coerced) — CONFIRMED, reproduced live.** `sell_parser`'s second sub-parser maps `Many::some_spaced(trade_good_parser())` to `Command::Sell { good: goods.first()..., quantity: goods.len() }` (command.rs:76-85), discarding every type after the first. Reproduced live: `parse("sell dia gold lea")` yields `Sell { good: Diamond, quantity: 3 }` with `remaining: ""`, and `command(player, "sell dia gold lea")` on a hand of 3 diamonds + 1 gold **succeeds**, logging `"sold 3 diamonds for 19 points and took a bonus token"` and leaving the hand as `[Gold]`. That is a silent, unintended, irreversible sale (`can_undo: false`, lib.rs:766). `sell()` itself cannot fix this: it receives only `good` and `quantity` (lib.rs:485-490) and never sees the type list, so the rejection must happen in the parser.
  The finding's literal recommendation ("in the `Map` closure, validate ... and fail the parse otherwise") is **impossible as written**: `Map`'s closure signature is `Fn(T) -> O`, not `-> Result<O, _>` (`rust/lib/game/src/command/parser/mod.rs:188-206`), and `Map::parse` unconditionally wraps the closure result in `Ok` (mod.rs:213-220). There is no fallible map combinator in `rust/lib/game/src/command/parser` (the full set is `Token`, `Int`, `Map`, `Opt`, `Many`, `Space`, `OneOf`, `Enum`, `Doc`, `Player`, `AfterSpace`, plus `Chain2/3/4`). So the fix is a small crate-local `Parser` impl that wraps the `Many` and validates — see Task 2. Error routing was checked: `OneOf` keeps the error(s) with the **largest** `offset` (mod.rs:471-486), `Chain2` propagates inner errors unmodified (`chain.rs:17-18`), and `GameError::Parse`'s `Display` is `"{message}, expected {…}"` (`rust/lib/game/src/errors.rs:21-26`), so setting `offset` to the consumed length makes the new message win over both the sibling `Int` sub-parser (offset 0) and the top-level `take_parser` (offset 0) and reach the player.
- **F20 (silent `unwrap_or(Good::Diamond)`) — CONFIRMED, unreachable but fragile.** `Many::some_spaced` sets `min: Some(1)` (mod.rs:310-317) and errors below the minimum (mod.rs:382-391), so `goods.first()` is always `Some`. The fallback at command.rs:79 would silently turn a parser regression into diamond sales. F20 lives in the same closure as F18, so both are retired by Task 2's rewrite. The finding offers "index `goods[0]`" as an option — **rejected**: indexing would trade a silent wrong answer for a panic on a player-reachable path, which repo rules forbid. Task 2 uses `let … else` returning a parse error instead: loud, non-panicking.
- **F19 (dead `is_empty` branch) — CONFIRMED.** `command_parser` (command.rs:12-24) unconditionally pushes the take and sell parsers (command.rs:17-18) before testing `parsers.is_empty()` (command.rs:19), so the `None` arm is dead; the real `None` condition is the early return at command.rs:13-15. No behaviour change available to test — covered by the existing `command_parser_returns_none_when_game_finished` (lib.rs:959-965) and `command_parser_returns_none_for_wrong_player` (lib.rs:967-973).
- **F22 ("N rounds remaining" overstates) — CONFIRMED.** `common_rows` computes `remaining_rounds = 3 - (round_wins[0] + round_wins[1])` (render.rs:174) and renders `"There {is|are} {n} {round|rounds} remaining."` (render.rs:184-189), but the match ends at the first player to 2 wins (`is_finished`, lib.rs:648-650; also `DATA_DOCS.md:6`). At 1-0 it says "There are 2 rounds remaining" while the match may well end after one more. A numeric round *index* is also not derivable from `round_wins`: a fully tied round is replayed **without** incrementing either counter (lib.rs:632-636 plus `full_tie_replays_round`, lib.rs:1172-1184), so any "round N of 3" wording would drift too. The fix therefore drops the numeric claim entirely, as the finding's second option suggests, and moves the (correct) win counts into the adjacent leader row so no information is lost.
- **F17 (RULES.md stub) — CONFIRMED.** `RULES.md` is exactly one line, `# Jaipur`; `rules()` serves it via `include_str!` (lib.rs:816-818) while `DATA_DOCS.md` (26 lines), `BASIC_STRATEGY.md` (28) and `ADVANCED_STRATEGY.md` (39) are substantial. The new RULES.md must describe the **implemented** rules only, and must stay silent on the three WP-26 questions (next-round starter, whether the camel token counts as a bonus token, camel visibility) so that neither WP-26's decision nor this document has to be rewritten.

---

### Task 1: award the bonus token for 6- and 7-card sales (d F14, major)

**Problem (restated):** `sell()` looks up the bonus pile with the raw sale size (`self.bonuses.get_mut(&quantity)`, lib.rs:521), and the map only has keys 3, 4, 5 (lib.rs:242-247). A 6- or 7-card sale — legal under `HAND_SIZE = 7` and reachable with cloth/spice/leather (and 6 for the rare goods) — silently gets no bonus token, contradicting the official rulebook, the crate's own "5 or more" column label (render.rs:153) and `DATA_DOCS.md:13`. Reproduced live: a 6-leather sale leaves the 5-pile untouched.

**Fix (re-derived):** clamp the lookup key to `MAX_TRADE_BONUS` (the existing constant, lib.rs:24) so any sale of 5 or more draws from the 5-pile. One-line change plus a comment. `quantity` itself is NOT clamped — the log text, the goods-token count and the hand removal all continue to use the true quantity.

**Edge cases:**
- Quantities 1 and 2 must still get nothing: `1.min(5) = 1` and `2.min(5) = 2`, neither is a key in `bonuses`, so `get_mut` returns `None` — behaviour unchanged.
- Quantities 3, 4, 5 are below or at the clamp, so they are byte-for-byte unchanged (this is what keeps `sell_with_bonus_includes_private_bonus_log`, lib.rs:1210-1221, passing).
- An exhausted 5-pile: `bonuses.first()` is `None`, so the `let`-chain at lib.rs:521-523 falls through and no bonus is awarded — correct, and the reason the F14 test must assert on the pile length rather than merely on `bonus_tokens`.
- `num_tokens = quantity.min(goods_pile.len())` (lib.rs:513) already handles a 6-card sale against a shorter token pile; untouched.
- Round-end interaction: a 6-leather sale takes 6 of the 9 leather tokens and a 7-spice sale empties the 7-spice pile, which is only 1 of the 3 piles needed for `emptied >= 3` (lib.rs:567-571), so neither test sale triggers `end_round`. Confirmed live.
- No serialized shape changes: `bonuses: HashMap<usize, Vec<u32>>` keeps its 3/4/5 keys.

**Files:**
- Modify: `rust/game/jaipur-2/src/lib.rs` (`sell()` at lines 519-529; tests in the inline `mod tests`)

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/jaipur-2/src/lib.rs` (append at the end of the module, before the closing brace at lib.rs:1553):

```rust
    #[test]
    fn sell_six_takes_bonus_from_the_five_or_more_pile() {
        // d F14: bonus piles exist only for sale sizes 3/4/5, but the 7-card
        // hand limit makes 6- and 7-card sales ordinary play. Official rules
        // and this crate's own renderer ("5 or more", render.rs) say such a
        // sale takes a token from the 5-sale pile.
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Leather; 6];
        let pile_before = g.bonuses[&MAX_TRADE_BONUS].len();
        g.sell(player, Good::Leather, 6).unwrap();
        assert_eq!(
            g.bonuses[&MAX_TRADE_BONUS].len(),
            pile_before - 1,
            "a 6-card sale must consume a token from the 5-or-more bonus pile"
        );
        assert_eq!(g.bonus_tokens[player], 1, "the bonus token must be counted");
        // 6 leather tokens (4+3+2+1+1+1) plus the one bonus token.
        assert_eq!(g.tokens[player].len(), 7);
    }

    #[test]
    fn sell_seven_takes_bonus_from_the_five_or_more_pile() {
        // d F14: the maximum legal sale (a full hand of 7) must also earn a
        // bonus token rather than falling off the end of the map.
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Spice; 7];
        let pile_before = g.bonuses[&MAX_TRADE_BONUS].len();
        g.sell(player, Good::Spice, 7).unwrap();
        assert_eq!(g.bonuses[&MAX_TRADE_BONUS].len(), pile_before - 1);
        assert_eq!(g.bonus_tokens[player], 1);
    }

    #[test]
    fn small_sales_still_earn_no_bonus_token() {
        // Guard for d F14's clamp: clamping must not start handing bonus
        // tokens to sales below the 3-card minimum.
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Leather, Good::Leather];
        g.sell(player, Good::Leather, 2).unwrap();
        assert_eq!(g.bonus_tokens[player], 0);
        assert_eq!(g.bonuses[&MIN_TRADE_BONUS].len(), 7);
        assert_eq!(g.bonuses[&MAX_TRADE_BONUS].len(), 5);
    }
```

- [ ] Run: `cargo test -p jaipur-2 sell_six sell_seven small_sales` — expected: `sell_six_takes_bonus_from_the_five_or_more_pile` and `sell_seven_takes_bonus_from_the_five_or_more_pile` FAIL on the `pile_before - 1` assertion (`left: 5, right: 4`, i.e. the pile was never touched); `small_sales_still_earn_no_bonus_token` PASSES already (it is the guard, not the red test).
- [ ] Implement: in `sell()` in `rust/game/jaipur-2/src/lib.rs`, replace lines 519-521:

```rust
        let mut suffix = String::new();
        let mut bonus_taken: Option<u32> = None;
        if let Some(bonuses) = self.bonuses.get_mut(&quantity)
```

  with:

```rust
        let mut suffix = String::new();
        let mut bonus_taken: Option<u32> = None;
        // Bonus piles only exist for sale sizes 3, 4 and 5, but the 7-card hand
        // limit makes 6- and 7-card sales legal; those take from the 5-pile,
        // which is why the renderer labels that column "5 or more".
        let bonus_key = quantity.min(MAX_TRADE_BONUS);
        if let Some(bonuses) = self.bonuses.get_mut(&bonus_key)
```

  Leave the rest of the block (lib.rs:522-529) exactly as it is — in particular `bonuses.remove(0)` and `self.bonus_tokens[player] += 1` are unchanged, and `quantity` continues to drive the log text at lib.rs:536-544 and the hand removal at lib.rs:561-565.
- [ ] Run: `cargo test -p jaipur-2` — the three new tests PASS, all 61 pre-existing tests PASS.
- [ ] `cargo clippy -p jaipur-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/jaipur-2/src/lib.rs` ; message: `fix(jaipur-2): award a bonus token for 6- and 7-card sales (d F14, WP-23)`

NOTE: this task shifts lib.rs line numbers below ~line 520 by +4; later tasks cite `sell()` and the tests module by symbol.

---

### Task 2: reject mixed-type sell input and drop the silent Diamond fallback (d F18 minor + d F20 nit)

**Problem (restated):** `sell_parser`'s bare-goods sub-parser (command.rs:76-85) keeps only `goods.first()` and uses `goods.len()` as the quantity, so `sell dia gold lea` parses as `Sell { Diamond, 3 }`. Reproduced live: with 3 diamonds and 1 gold in hand, `command(player, "sell dia gold lea")` **succeeds**, sells 3 diamonds for 19 points plus a bonus token, and cannot be undone. The same closure also defaults an empty list to `Good::Diamond` via `unwrap_or` (command.rs:79) — currently unreachable (`Many::some_spaced` has `min: Some(1)`), but a silent arbitrary default would mask a parser regression as diamond sales.

**Fix (re-derived):** the finding's "fail the parse inside the `Map` closure" is not expressible — `Map`'s closure is infallible (`rust/lib/game/src/command/parser/mod.rs:188-206`) and the shared parser library has no fallible map. So replace the `Map` with a small crate-local `Parser` implementation, `SellGoodsParser`, that wraps the same `Many` and:
1. errors if the parsed types are not all identical (F18), with an `offset` equal to the consumed length so `OneOf` surfaces the message (mod.rs:471-486);
2. errors — rather than panicking or defaulting — if the list is somehow empty (F20).

`to_spec()` and `expected()` delegate straight to the inner `Many`, so the client-facing `CommandSpec` from `command_spec()` (lib.rs:798-800) and all autocomplete behaviour are **unchanged**. `trade_good_parser()`'s return type is narrowed from `impl Parser<T = Good>` to the concrete `Enum<Good>` so the new struct can name its field type; `Enum<Good>` is what `Enum::partial` already returns (mod.rs:551-577) and `Good: Display + Clone` satisfies its `ToString + Clone` bound.

**Edge cases:**
- `sell 2 gold` and `sell 12 lea` keep working: they are handled by the sibling `Int`-based sub-parser `p1` (command.rs:69-75), which this task does not touch.
- `sell dia dia` (repeated same type) must still parse to `Sell { Diamond, 2 }` — all elements equal, no error.
- Trailing input must still be preserved: for `sell 2 gold and then`, `p1` matches and `remaining_input` stays `" and then"` (`command_preserves_remaining_input`, lib.rs:1544-1552). For a bare-goods form like `sell lea lea and then`, `Many` stops at `"and"` (its element parser errors and the loop breaks, mod.rs:355-375), so `goods = [Leather, Leather]`, all equal — parses, remainder preserved. Verified by reading `Many::parse`.
- After this change, mixed input is a hard parse error instead of falling back to "sell just the first good". That is the point: the old fallback silently executed a different, irreversible action.
- Empty-list branch: unreachable today, so it gets no dedicated test (a test would have to bypass `Many`'s own minimum). It is a `let … else` returning `GameError::Parse`, never a panic.
- `Command` is not serialized (command.rs:5 derives only `Debug, PartialEq, Clone`), and its variants are unchanged anyway, so no persisted shape moves.
- `sell()` (lib.rs:485-577) is deliberately left alone: it never receives the type list, so it cannot detect mixed sales.

**Files:**
- Modify: `rust/game/jaipur-2/src/command.rs` (imports line 1-3, `trade_good_parser` lines 31-33, new `SellGoodsParser`, `sell_parser` lines 76-85)
- Test: `rust/game/jaipur-2/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/jaipur-2/src/lib.rs`:

```rust
    #[test]
    fn sell_parser_rejects_mixed_good_types() {
        // d F18: `sell dia gold lea` used to parse as Sell { Diamond, 3 } -
        // the type list after the first entry was silently discarded.
        let g = Game::start(2, 0).unwrap().0;
        let parser = g.command_parser(g.current_player).unwrap();
        let err = parser
            .parse("sell dia gold lea", &[])
            .expect_err("a mixed-type sell must not parse");
        assert!(
            err.to_string().contains("only sell one type of good"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn command_rejects_mixed_type_sell_without_selling_anything() {
        // d F18: the real damage is at the command level - the coerced parse
        // executed an irreversible sale the player never asked for.
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Diamond, Good::Diamond, Good::Diamond, Good::Gold];
        let hand_before = g.hands[player].clone();
        let tokens_before = g.tokens[player].clone();
        assert!(g.command(player, "sell dia gold lea", &[]).is_err());
        assert_eq!(
            g.hands[player], hand_before,
            "no cards may leave the hand on a rejected sale"
        );
        assert_eq!(g.tokens[player], tokens_before);
    }

    #[test]
    fn sell_parser_still_parses_repeated_same_good() {
        // Regression guard for d F18's fix: the legitimate bare-goods form
        // must keep working.
        let g = Game::start(2, 0).unwrap().0;
        let parser = g.command_parser(g.current_player).unwrap();
        let output = parser.parse("sell dia dia", &[]).unwrap();
        match output.value {
            Command::Sell { good, quantity } => {
                assert_eq!(good, Good::Diamond);
                assert_eq!(quantity, 2);
            }
            _ => panic!("expected Sell command"),
        }
    }
```

- [ ] Run: `cargo test -p jaipur-2 mixed` — expected FAILURES: `sell_parser_rejects_mixed_good_types` fails on `expect_err` (the parse succeeds today), and `command_rejects_mixed_type_sell_without_selling_anything` fails on `is_err()` (the command succeeds and empties three diamonds out of the hand). `sell_parser_still_parses_repeated_same_good` PASSES already (guard).
- [ ] Implement, in `rust/game/jaipur-2/src/command.rs`:
  1. Extend the imports (lines 1-3) to:

```rust
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::*;
use brdgme_game::errors::GameError;

use crate::{Game, Good};
```

  (`CommandSpec` and `GameError` are used privately inside the parser module and are **not** re-exported by the `parser::*` glob, so both need naming explicitly.)
  2. Narrow `trade_good_parser` (lines 31-33) to the concrete type so it can be used as a struct field:

```rust
fn trade_good_parser() -> Enum<Good> {
    Enum::partial(Good::trade_goods().to_vec())
}
```

  (This is the type `Enum::partial` already returns; the existing call site at command.rs:70 is unaffected because `Enum<Good>: Parser<T = Good>`.)
  3. Add the new parser, immediately above `sell_parser`:

```rust
/// Parses the bare-goods form of a sell command, eg. `sell dia dia`.
///
/// This exists instead of a `Map` over `Many` because `Map`'s closure is
/// infallible, and two things here must be able to fail the parse:
/// mixed good types (which used to be silently truncated to the first type,
/// executing an unintended sale) and an empty list (which used to fall back
/// to `Good::Diamond`, hiding any parser regression).
struct SellGoodsParser {
    inner: Many<Enum<Good>, Space>,
}

impl Parser for SellGoodsParser {
    type T = Command;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Command>, GameError> {
        let out = self.inner.parse(input, names)?;
        let Some(&good) = out.value.first() else {
            // Unreachable while `inner` is a `some_spaced` Many, but stated as
            // an error rather than an unwrap or a default so a regression is
            // loud and never a panic on a player-reachable path.
            return Err(GameError::Parse {
                message: Some("you must name at least one good to sell".to_string()),
                expected: self.expected(names),
                offset: 0,
            });
        };
        if let Some(other) = out.value.iter().find(|&&g| g != good) {
            return Err(GameError::Parse {
                message: Some(format!(
                    "you can only sell one type of good at a time, got {good} and {other}"
                )),
                expected: self.expected(names),
                // A non-zero offset makes OneOf prefer this message over the
                // sibling "sell <n> <good>" parser's offset-0 failure.
                offset: out.consumed.len(),
            });
        }
        Ok(Output {
            value: Command::Sell {
                good,
                quantity: out.value.len(),
            },
            consumed: out.consumed,
            remaining: out.remaining,
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.inner.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        self.inner.to_spec()
    }
}
```

  4. In `sell_parser`, replace the `p2` binding (command.rs:76-85):

```rust
                let p2: Box<dyn Parser<T = Command>> = Box::new(Map::new(
                    Many::some_spaced(trade_good_parser()),
                    |goods: Vec<Good>| {
                        let good = goods.first().copied().unwrap_or(Good::Diamond);
                        Command::Sell {
                            good,
                            quantity: goods.len(),
                        }
                    },
                ));
```

  with:

```rust
                let p2: Box<dyn Parser<T = Command>> = Box::new(SellGoodsParser {
                    inner: Many::some_spaced(trade_good_parser()),
                });
```

- [ ] Run: `cargo test -p jaipur-2` — the three new tests PASS, all previous tests PASS (in particular `sell_parser_parses_quantity_prefix`, `command_preserves_remaining_input`, and `game_contract`, whose `CommandSpec` round-trip is preserved by the `to_spec` delegation).
- [ ] `cargo clippy -p jaipur-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean. If clippy flags the now-unused `Map` import, note that `Map` is still used by `take_parser` (command.rs:37) and by `p1` (command.rs:69) — it comes from the glob import, so there is nothing to remove.
- [ ] Commit: `git add rust/game/jaipur-2/src/command.rs rust/game/jaipur-2/src/lib.rs` ; message: `fix(jaipur-2): reject mixed-type sell input instead of coercing it (d F18 d F20, WP-23)`

---

### Task 3: delete the dead `parsers.is_empty()` branch (d F19, nit)

**Problem (restated):** `command_parser` (command.rs:12-24) returns early with `None` when the game is finished or it is not this player's turn (command.rs:13-15), then unconditionally pushes both parsers (command.rs:17-18) before testing `parsers.is_empty()` (command.rs:19). The `None` arm of that test can never be taken; it invites a reader to believe there is a third no-commands-available state.

**Fix:** build the vec directly and return `Some(...)` unconditionally. No behaviour change — the existing tests `command_parser_returns_none_when_game_finished` (lib.rs:959-965), `command_parser_returns_none_for_wrong_player` (lib.rs:967-973) and `command_spec_returns_none_for_wrong_player` (lib.rs:1492-1500) already pin the two real `None` paths, so no new test is written (there is nothing observable to assert).

**Files:**
- Modify: `rust/game/jaipur-2/src/command.rs` (`command_parser`, locate by symbol after Task 2)

**Steps:**

- [ ] Replace the body after the early return (command.rs:16-23):

```rust
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        parsers.push(Box::new(take_parser()));
        parsers.push(Box::new(sell_parser()));
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
```

  with:

```rust
        let parsers: Vec<Box<dyn Parser<T = Command>>> =
            vec![Box::new(take_parser()), Box::new(sell_parser())];
        Some(Box::new(OneOf::new(parsers)))
```

  The early return at command.rs:13-15 is the only `None` path and must stay exactly as it is.
- [ ] Run: `cargo test -p jaipur-2` — full suite PASSES.
- [ ] `cargo clippy -p jaipur-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/jaipur-2/src/command.rs` ; message: `refactor(jaipur-2): drop the dead empty-parsers branch (d F19, WP-23)`

---

### Task 4: stop claiming a fixed number of remaining rounds (d F22, nit)

**Problem (restated):** `common_rows` (render.rs:173-200) renders `"There {is|are} {3 - wins} {round|rounds} remaining."` from `remaining_rounds` (render.rs:174). The match is first-to-2 (lib.rs:648-650), so at 1-0 the claim "There are 2 rounds remaining" is wrong whenever the leader wins again — the common case. A round-index wording is not available either: a fully tied round is replayed without incrementing `round_wins` (lib.rs:632-636, `full_tie_replays_round` lib.rs:1172-1184), so any counter derived from `round_wins` drifts.

**Fix (re-derived):** replace the numeric-claim row with the match condition ("First to 2 round wins takes the game."), and fold the actual win counts — the only information the old row carried indirectly — into the adjacent leader row, which currently says only who leads. Net effect: no false claim, no information lost.

**Edge cases:**
- The leader row keeps its existing literal `"Player 0"` / `"Player 1"` phrasing (see "Cross-package / newly discovered" below — switching those to `N::Player` markup nodes is a separate, unfiled issue and is **not** part of this task).
- 0-0 start: renders "Round wins are level at 0 - 0." — accurate.
- Finished match (2-0): renders "First to 2 round wins takes the game." plus "Player 0 leads 2 - 0." — accurate; the placings log carries the result.
- `pluralize` (render.rs:44-46) stays in use (deck count, camels, tokens, goods), so removing its use here does not create a dead function.
- `remaining_rounds` was the only use of `saturating_sub` in `common_rows`; deleting the local removes the need for it.
- Do NOT touch `camel_display` / `opponent_rows` in the same file — d F23, owned by WP-26.

**Files:**
- Modify: `rust/game/jaipur-2/src/render.rs` (`common_rows`, lines 173-199)
- Test: `rust/game/jaipur-2/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/jaipur-2/src/lib.rs`:

```rust
    #[test]
    fn render_does_not_claim_a_fixed_number_of_remaining_rounds() {
        // d F22: at 1-0 the renderer said "There are 2 rounds remaining", but
        // the match ends the moment a player reaches 2 round wins.
        use brdgme_game::Renderer;
        let ps = PubState {
            round_wins: [1, 0],
            ..PubState::default()
        };
        let markup = brdgme_markup::to_string(&ps.render());
        assert!(
            !markup.contains("rounds remaining"),
            "must not claim a number of remaining rounds, got: {markup}"
        );
        assert!(
            markup.contains("First to 2 round wins"),
            "expected the first-to-2 wording, got: {markup}"
        );
        assert!(
            markup.contains("1 - 0"),
            "the round-win counts must still be visible, got: {markup}"
        );
    }
```

- [ ] Run: `cargo test -p jaipur-2 render_does_not_claim` — expected FAIL on the first assertion (`markup` contains "There are 2 rounds remaining.").
- [ ] Implement: in `rust/game/jaipur-2/src/render.rs`, replace lines 173-190 (the head of `common_rows` through the leader row) with:

```rust
fn common_rows(pub_state: &PubState) -> Vec<Row> {
    let (w0, w1) = (pub_state.round_wins[0], pub_state.round_wins[1]);
    // No "N rounds remaining": the match ends at the first player to reach 2
    // round wins, and a fully tied round is replayed without incrementing
    // either counter, so no count derived from round_wins is trustworthy.
    let leader_text = if w0 > w1 {
        format!("Player 0 leads {w0} - {w1}.")
    } else if w1 > w0 {
        format!("Player 1 leads {w1} - {w0}.")
    } else {
        format!("Round wins are level at {w0} - {w1}.")
    };

    vec![
        centered_row(vec![N::Bold(vec![N::text(
            "First to 2 round wins takes the game.",
        )])]),
        centered_row(vec![N::text(leader_text)]),
```

  Everything from `blank_row(),` (render.rs:191) to the end of the function stays exactly as it is.
- [ ] Run: `cargo test -p jaipur-2` — the new test PASSES, all previous tests PASS (`pub_state_renders_without_panicking`, `player_state_renders_without_panicking`, `player_state_renders_own_hand`, `token_table_renders_bottom_up_source_order` assert only on non-emptiness, the `"You have"` heading and token-table ordering).
- [ ] `cargo clippy -p jaipur-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/jaipur-2/src/render.rs rust/game/jaipur-2/src/lib.rs` ; message: `fix(jaipur-2): render the first-to-2 match condition instead of a wrong round count (d F22, WP-23)`

---

### Task 5: write a real RULES.md (d F17, minor, doc-only) + final gate

**Problem (restated):** `RULES.md` is the single line `# Jaipur`, and `rules()` (lib.rs:816-818) `include_str!`s it, so the in-game rules page is effectively empty while the crate ships three substantial strategy/data documents. The contract harness only checks that rules are non-empty (`rust/lib/cmd/src/test_support.rs:32-37`), so the stub passes CI today.

**Fix (re-derived):** write RULES.md describing the **implemented** rules, verified line by line against `src/lib.rs`. Two hard constraints on the content:
1. **It must document the 8-camels-in-deck + 3-camels-placed-in-market split explicitly** (lib.rs:105 and lib.rs:223). That split is exactly what the rejected d F13 misread as a missing-camels bug; writing it down is the durable fix for that class of review error.
2. **It must stay silent on the three WP-26 questions** — who starts the next round, whether the camel token counts towards the "most bonus tokens" tie-break, and how much of an opponent's camel herd is visible — so that WP-26's decision does not invalidate this file (and vice versa).

Every claim below was checked against source: components (lib.rs:97-107, 117-127, 136-143, `CAMEL_BONUS_POINTS` lib.rs:22), setup (lib.rs:221-247), the take actions (lib.rs:317-348, 350-479), hand limit 7 (lib.rs:21, 386-390), minimum sales (lib.rs:109-115, 497-503), no camel sales (lib.rs:494-496), token draw from the front of the pile (lib.rs:512-517), the private bonus-value log (lib.rs:554-559), round-end triggers (lib.rs:257-260, 567-575), camel bonus (lib.rs:580-599), scoring and tie-breaks (lib.rs:601-627), tie replay (lib.rs:632-636), and match end (lib.rs:648-650).

**Edge cases:**
- After Task 1, "any sale of 5 or more takes from the 5-sale pile" is a true statement; before Task 1 it is not — so **this task must run after Task 1**.
- The wording "the value of a bonus token is shown privately to the seller" matches lib.rs:554-559. Do not claim the value stays secret in general: `points()` (lib.rs:802-806) exposes each player's running token total through the framework.
- `DATA_DOCS.md`, `BASIC_STRATEGY.md` and `ADVANCED_STRATEGY.md` are already accurate on everything this package changes (`DATA_DOCS.md:13` already says bonus tokens are for "selling 3+") — no edits needed there.
- `rules()` is `include_str!`, so the only build effect is a re-embed.

**Files:**
- Modify: `rust/game/jaipur-2/RULES.md` (replace the entire file)

**Steps:**

- [ ] Replace the full contents of `rust/game/jaipur-2/RULES.md` with:

```markdown
# Jaipur

Jaipur is a two-player game of trading goods in the markets of Rajasthan. Buy,
exchange and sell goods at the best moment, and keep the largest camel herd.

A match is played over up to three rounds. The first player to win two rounds
wins the match.

## Components

- A deck of 52 cards: 6 diamond, 6 gold, 6 silver, 8 cloth, 8 spice,
  10 leather and 8 camel.
- One goods token pile per trade good, highest value first:
  - diamond: 7 7 5 5 5
  - gold: 6 6 5 5 5
  - silver: 5 5 5 5 5
  - cloth: 5 3 3 2 2 1 1
  - spice: 5 3 3 2 2 1 1
  - leather: 4 3 2 1 1 1 1 1 1
- Three shuffled, face-down bonus token piles:
  - for 3-card sales: 3 3 2 2 2 1 1
  - for 4-card sales: 6 6 5 5 4 4
  - for sales of 5 or more cards: 10 10 9 8 8
- One camel token worth 5 points.

## Setup

Each round is set up from scratch:

- Three camels are placed directly into the market. They are **not** dealt from
  the deck, so 11 camels are in play in total: 8 in the 52-card deck plus these
  3 in the market.
- The deck is shuffled and two more cards are drawn into the market, so the
  market holds 5 cards.
- Each player is dealt 5 cards. Any camels dealt to a player go straight into
  that player's herd and do not count against the hand limit.
- 40 cards remain in the deck.

## Your turn

On your turn you either take cards or sell cards.

### Taking

- `take <good>` - take one good from the market into your hand. You may not put
  anything back.
- `take <good> <good> ... for <good> <good> ...` - exchange two or more cards.
  You must give back exactly as many cards as you take. The cards you give come
  from your hand or from your camel herd, and none of them may be the same type
  as any card you take.
- `take camel` - take **all** the camels in the market into your herd.

Your hand may never hold more than 7 goods. Camels live in your herd, not your
hand, so they are not limited.

After any take, the market is refilled from the deck back up to 5 cards.

### Selling

- `sell <n> <good>` or `sell <good> <good> ...` - sell cards from your hand.
- Every card in one sale must be the same type of good.
- Diamond, gold and silver require a minimum sale of 2 cards. Cloth, spice and
  leather can be sold one at a time.
- Camels can never be sold.
- Take one goods token per card sold, from the top (highest remaining value) of
  that good's pile. If the pile runs out you simply take fewer tokens.
- Selling 3 or more cards at once also earns one bonus token: from the 3-card
  pile for a 3-card sale, the 4-card pile for a 4-card sale, and the
  5-or-more pile for any sale of 5 or more cards. The value of the bonus token
  is shown privately to the seller.

## End of a round

A round ends immediately when either:

- three goods token piles have been exhausted, or
- the deck can no longer refill the market to 5 cards.

Then:

- The player with more camels takes the 5 point camel token. If both players
  have the same number of camels, nobody takes it.
- Each player adds up the value of every token collected during the round.
- The higher total wins the round. If the totals are equal, the round goes to
  the player with the most bonus tokens; if that is also equal, to the player
  with the most goods tokens. If everything is equal the round is replayed.

## Winning the match

The first player to win two rounds wins the match.
```

  Do NOT add anything about which player starts a later round, whether the
  camel token counts as a bonus token for the tie-break, or how much of the
  opponent's herd is visible — WP-26 owns all three.
- [ ] Run: `cargo test -p jaipur-2` — full suite PASSES, including `game_contract` (its non-empty-rules assertion is now satisfied by real content).
- [ ] `cargo clippy -p jaipur-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/jaipur-2/RULES.md` ; message: `docs(jaipur-2): replace the RULES.md stub with the implemented rules (d F17, WP-23)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| d F13 deck has 8 camels / 52 cards | major | Change `Good::Camel => 8` to `=> 11`, update `deck_has_52_cards` and `start_deck_is_40` | **OVERTURNED (REJECTED upstream)** | Re-traced live: `start_round` places 3 market camels without drawing them (lib.rs:223), so 8 + 3 = 11 camels are in play and the deck lands at 40 exactly as official (`start_deck_is_40` passes). The recommended fix would create 14 camels. Not in this package's scope, explicitly listed as a non-goal, and documented in Task 5's RULES.md so it is not re-filed. |
| d F14 no bonus token for 6/7-card sales | major | `let key = quantity.min(5);` plus a regression test selling 6+ leather | **CONFIRMED (adjusted detail)** | Defect reproduced live (6-leather sale left the 5-pile at 5 and `bonus_tokens` at 0). Fix uses the existing `MAX_TRADE_BONUS` constant (lib.rs:24) rather than a literal `5`, and adds three tests: 6-card, 7-card, and a guard that 1/2-card sales still earn nothing (Task 1). |
| d F15 next-round starter is not the round loser | major -> minor (verification) | Track the loser in `end_round` and set `current_player` | **OUT OF SCOPE** | Owned by WP-26 (BLOCKED-ON-DECISION); the "loser starts" premise has no in-repo source. `end_round`, `next_player` and the sell-triggered round-end branch are untouched, and RULES.md deliberately says nothing about it. |
| d F16 camel token counted as a bonus token | minor | Track the camel token separately from `bonus_tokens` | **OUT OF SCOPE** | Owned by WP-26. lib.rs:598 and the tie-break chain (lib.rs:613-627) untouched; Task 5's tie-break wording is deliberately agnostic about whether the camel token counts. |
| d F17 RULES.md is a one-line stub | minor | Write a real RULES.md (setup, actions, bonuses, camel bonus, round end, best-of-3) | **CONFIRMED (scope tightened)** | Written from the live implementation, claim by claim. Two additions beyond the recommendation: it documents the 8-deck/3-market camel split so d F13's misreading cannot recur, and it is deliberately silent on the three WP-26 questions so neither document has to be rewritten (Task 5). |
| d F18 mixed-type sell silently coerced | minor | Validate `goods.iter().all(...)` inside the `Map` closure and fail the parse, or reject mixed sales in `sell()` | **ADJUSTED (both offered mechanisms rejected)** | Defect reproduced live and is worse than "a confusing error": `command(p, "sell dia gold lea")` sells 3 diamonds for 19 points plus a bonus, un-undoable. But (a) `Map`'s closure is `Fn(T) -> O` and cannot fail (`lib/game/src/command/parser/mod.rs:188-220`) and there is no fallible map in the library, and (b) `sell()` only receives `good` + `quantity` (lib.rs:485-490) and never sees the type list, so it *cannot* detect mixed sales. Fix is a small crate-local `SellGoodsParser` implementing `Parser`, with `to_spec`/`expected` delegated so the client-facing `CommandSpec` is unchanged (Task 2). |
| d F19 dead `parsers.is_empty()` branch | nit | `Some(Box::new(OneOf::new(parsers)))` | **CONFIRMED** | Exactly as recommended, plus collapsing the two `push` calls into the `vec!` literal. No test: nothing observable; the two real `None` paths are already pinned by three existing tests (Task 3). |
| d F20 silent `unwrap_or(Good::Diamond)` | nit | Index `goods[0]` or destructure to enforce the non-empty invariant loudly | **ADJUSTED** | Confirmed unreachable (`Many::some_spaced` sets `min: Some(1)`, mod.rs:310-317, errors at 382-391). The "index `goods[0]`" option is **rejected**: it converts a silent wrong answer into a panic on a player-reachable path, which repo rules forbid. Task 2 uses `let Some(&good) = … else { return Err(GameError::Parse …) }` — loud, non-panicking, and retired in the same rewrite as F18. |
| d F21 placings-log block duplicated | nit | Collapse the two match arms and share the is_finished block | **OUT OF SCOPE** | work-packages.md:81 assigns `d F21` to the cross-crate epilogue/placings dedup package, not WP-23. `command()` (lib.rs:725-796) untouched. |
| d F22 "N rounds remaining" overstates | nit | Render "first to 2 round wins" or reword to avoid a numeric claim | **CONFIRMED (adjusted)** | Takes the finding's first option. Additionally moves the real win counts into the adjacent leader row so the row change loses no information; a "round N of 3" alternative was **rejected** because tied rounds are replayed without incrementing `round_wins` (lib.rs:632-636), so any such counter drifts (Task 4). |
| d F23 opponent camel display leaks exact zero | nit | Pick one policy: exact counts in the renderer, or clamp in `PubState` too | **OUT OF SCOPE** | Owned by WP-26. `camel_display` (render.rs:40-42) and `opponent_rows` untouched even though Task 4 edits the same file. |

## Test plan summary

Run from `/home/beefsack/Development/brdgme/rust`. Per-crate only.

| Command | When | Expectation |
|---|---|---|
| `cargo test -p jaipur-2` | baseline, before any edit | 61 pass (60 inline + `game_contract`) |
| `cargo test -p jaipur-2 sell_six sell_seven small_sales` | Task 1, red | 2 fail on the bonus-pile assertion, 1 passes (guard) |
| `cargo test -p jaipur-2 mixed` | Task 2, red | 2 fail (parse/command succeed today) |
| `cargo test -p jaipur-2 render_does_not_claim` | Task 4, red | 1 fail ("There are 2 rounds remaining.") |
| `cargo test -p jaipur-2` | after each task, green | all pass; final count 61 + 7 new = **68** |
| `cargo clippy -p jaipur-2 --all-targets -- -D warnings` | after each task | clean |
| `cargo fmt --all -- --check` | after each task | clean |
| `/home/beefsack/Development/brdgme/scripts/rust-test.sh` | before the final commit only | pass |

New tests, all in the inline `#[cfg(test)] mod tests` of `rust/game/jaipur-2/src/lib.rs`:

1. `sell_six_takes_bonus_from_the_five_or_more_pile` (Task 1, red first)
2. `sell_seven_takes_bonus_from_the_five_or_more_pile` (Task 1, red first)
3. `small_sales_still_earn_no_bonus_token` (Task 1, guard)
4. `sell_parser_rejects_mixed_good_types` (Task 2, red first)
5. `command_rejects_mixed_type_sell_without_selling_anything` (Task 2, red first)
6. `sell_parser_still_parses_repeated_same_good` (Task 2, guard)
7. `render_does_not_claim_a_fixed_number_of_remaining_rounds` (Task 4, red first)

Tasks 3 (d F19) and 5 (d F17) add no tests: F19 has no observable behaviour change and its two real `None` paths are already covered; F17 is markdown whose non-emptiness is already asserted by `game_contract`.

Task order is: **1 → 2 → 3 → 4 → 5**. Task 5 must be last because its RULES.md sentence "any sale of 5 or more takes from the 5-sale pile" is only true after Task 1, and because it runs the full pre-commit gate.

## Cross-package / newly discovered

- **WP-26 (BLOCKED-ON-DECISION)** owns d F15, d F16 and d F23, all in this crate. Anticipated overlap: WP-26 will edit `end_round` (lib.rs:579-642) and `camel_display`/`opponent_rows` (render.rs:40-42, 224-247); this package touches `sell()`'s bonus lookup (lib.rs:519-521), `common_rows`' first two rows (render.rs:173-190), `command.rs`, and RULES.md. No overlapping lines. If WP-26 later decides the camel token is not a bonus token, or that the loser starts the next round, RULES.md (Task 5) will need one or two sentences **added** — it deliberately makes no claim either way, so nothing already written becomes wrong.
- **The cross-crate placings/epilogue dedup package** (work-packages.md:81, which lists `d F21`) owns `command()`'s duplicated is_finished blocks (lib.rs:754-764, 777-787). WP-23 does not touch `command()`.
- **Newly discovered (nit, NOT fixed here — needs routing):** `src/render.rs` never uses `N::Player` markup nodes. `common_rows`' leader row hardcodes the literal strings `"Player 0 is in the lead."` / `"Player 1 is in the lead."` (render.rs:175-181), so the rendered board shows a bare index where every log line in `src/lib.rs` shows the real, coloured player name via `N::Player` (lib.rs:301, 328, 442, 532, 589, 594, 605, 630). Task 4 rewrites that row's text but **deliberately preserves the literal wording** rather than silently changing player identity rendering. **ROUTED by the unit-3 Lead to WP-26** (batch-d rules/display adjudication), which already owns the other jaipur player-visible display policy item (d F23 camel visibility) and already plans to edit `src/render.rs`. Rationale: no renderer/markup-consistency package exists, this changes what players see, and WP-26 is BLOCKED-ON-DECISION anyway, so a display-policy call belongs there rather than being folded into a mechanical-fixes package. It is a two-node change (`vec![N::Player(0), N::text(" leads …")]`); WP-26's spec writer must re-derive it from the then-current `common_rows`, which Task 4 of this package rewrites. Do NOT fold it into WP-23.
- **Observed, not a defect (recorded so it is not re-filed):** `replenish_market` ends the round when the deck holds fewer cards than needed to refill the market to 5 (lib.rs:257-260) rather than refilling partially first. Verification recorded round-end triggers as clean and no finding covers it; left exactly as is.
