# Verification: games batch D (jaipur-2), F13-F23

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (commit f8763a5).
No Go jaipur source exists: `find brdgme-go -iname '*jaipur*'` returns nothing. RULES.md is the one-line stub `# Jaipur`.

## F13 Camel card_count 8 / 52-card deck

- verdict: REJECTED
- evidence: `game/jaipur-2/src/lib.rs:105` has `Good::Camel => 8` and `deck_has_52_cards` (lib.rs:838-840) asserts `initial_deck().len() == 52`. The reviewer is right that the official deck is 55 cards with 11 camels (evidence basis: external, my own knowledge agrees; other counts 6/6/6/8/8/10 match official). BUT the behavioral claim fails on the setup code: `start_round` (lib.rs:221-224) sets `self.market = vec![Good::Camel, Good::Camel, Good::Camel]` conjured out of thin air, NOT drawn from the deck, then replenishes 2 cards from the deck and deals 5 per player. Total camels in play = 8 (deck) + 3 (market) = 11, and the shuffled 52-card deck (8 camels + 44 goods) is exactly the official post-setup deck (55 minus the 3 camels seeded to the market). Post-setup deck size: 52 - 2 (market fill) - 10 (hands) = 40, matching official 55 - 3 - 2 - 10 = 40 and the `start_deck_is_40` test (lib.rs:913-916). The game is distributionally identical to official rules.
- The recommended "43" correction is arithmetically consistent only with changing card_count to 11 while keeping the conjured market camels (55 - 2 - 10 = 43), which would put 14 camels in play - that fix would introduce the very bug it claims to remove.
- severity: corrected - not major/correctness; at most nit/quality (the names `initial_deck`/`deck_has_52_cards` describe the post-setup deck and could mislead readers, as it misled the reviewer).
- evidence basis: external (official 55/11) for the rule; in-repo code for the setup equivalence.

## F14 No bonus token for 6- or 7-card sales

- verdict: CONFIRMED
- evidence: `lib.rs:521` `if let Some(bonuses) = self.bonuses.get_mut(&quantity)` - `bonuses` is keyed only 3,4,5 (`bonus_sizes()` = `MIN_TRADE_BONUS..=MAX_TRADE_BONUS` = 3..=5, lib.rs:23-24,145-147, populated at lib.rs:242-247). A sale of 6 or 7 gets no bonus token. Such sales are possible: HAND_SIZE = 7 (lib.rs:21) and leather has 10 cards; `sell` only checks `quantity >= min_sale` and `quantity <= in_hand` (lib.rs:497-510). Contradicted by the crate's own renderer, `render.rs:153` labels the 5-pile `"5 or more"`, and by DATA_DOCS.md:13 "Bonus tokens are awarded when selling 3+ of a good at once". Official rule (external, my knowledge agrees): 5+ card sales take a 5-card bonus token.
- severity: upheld - major/correctness (real scoring bug reachable in normal play; code contradicts its own docs/UI).
- evidence basis: external for the official 5+ rule; in-repo (render.rs:153, DATA_DOCS.md:13) for internal contradiction.

## F15 Round loser does not start the next round

- verdict: ADJUSTED
- evidence: code behavior confirmed exactly as claimed. `end_round` (lib.rs:579-642) never touches `current_player`; neither does `start_round` (lib.rs:213-250). Paths: (a) round ends via sell with 3 piles empty, lib.rs:571-575 calls `end_round` instead of `next_player`, so the seller starts the next round; (b) deck exhaustion in `take_camels` (lib.rs:343-345) and `take_goods` (lib.rs:474-476): `replenish_market` returns false after calling `end_round` and `next_player()` is skipped, so the player who took the round-ending action starts. In every path the acting player (usually the round winner) starts the next round.
- adjustment: the claim that the official rule is "the player who lost the previous round starts" cannot be corroborated from any in-repo source (RULES.md is a stub; strategy docs are silent), and from my own knowledge I am not certain the official rulebook explicitly specifies a next-round starting player (it is a commonly cited convention, e.g. on digital implementations). The code defect (never resetting/deciding the starter deliberately - it just falls out of which path ended the round) stands regardless.
- severity: corrected - downgrade major -> minor unless the loser-starts rule is confirmed from the actual rulebook; the deviation is real but its authority is unverified.
- evidence basis: in-repo for code behavior; external and uncertain for the rule claim.

## F16 Camel token counted as bonus token in tie-break

- verdict: CONFIRMED
- evidence: `lib.rs:597-598`: `self.tokens[cw].push(CAMEL_BONUS_POINTS); self.bonus_tokens[cw] += 1;`. `bonus_tokens` is the first tie-break at lib.rs:617-620 (after points, before `good_tokens` at 621-624). So the camel-bonus winner gets a tie-break edge from the camel token itself. Official rules (external, my knowledge agrees): the camel token is distinct from the 18 bonus tokens and the tie-break is "most bonus tokens". In-repo docs are silent on whether the camel token counts (BASIC_STRATEGY.md:28 just says "most bonus tokens, then most good tokens").
- severity: upheld - minor/correctness (affects only exact-tie rounds).
- evidence basis: external for camel-token/bonus-token distinction.

## F17 RULES.md is a stub

- verdict: CONFIRMED
- evidence: `game/jaipur-2/RULES.md` contains exactly `# Jaipur` (one line). `rules()` at lib.rs:816-818 serves it via `include_str!`. BASIC_STRATEGY.md, ADVANCED_STRATEGY.md, DATA_DOCS.md all exist with substantive content (verified by reading them), so the rules stub is an outlier.
- severity: upheld - minor/quality.
- evidence basis: in-repo.

## F18 Sell parser silently discards mixed goods

- verdict: CONFIRMED
- evidence: `command.rs:76-85`: second sell sub-parser is `Many::some_spaced(trade_good_parser())` mapped to `Sell { good: goods.first().copied().unwrap_or(Good::Diamond), quantity: goods.len() }`. `sell dia gold lea` parses to `Sell { Diamond, 3 }`; the gold/leather tokens are consumed by the parser (not left in remaining_input) and their identity discarded. `sell()` (lib.rs:504-510) only validates the player holds `quantity` of `good` - if the player happens to hold 3+ diamonds, the command succeeds and sells 3 diamonds, not what the user typed. If not, they get the misleading error "you only have N of that good" about diamonds.
- severity: upheld - minor/correctness (requires malformed user input, but the failure is silent when it succeeds).
- evidence basis: in-repo.

## F19 Dead is_empty branch in command_parser

- verdict: CONFIRMED
- evidence: `command.rs:16-23`: `parsers` receives two unconditional pushes (lines 17-18) immediately before `if parsers.is_empty() { None }`, so the None arm is unreachable. The genuine None case (finished game / wrong player) is already returned at command.rs:13-15.
- severity: upheld - nit/simplicity.
- evidence basis: in-repo.

## F20 Unreachable unwrap_or(Good::Diamond)

- verdict: CONFIRMED
- evidence: `command.rs:79` `goods.first().copied().unwrap_or(Good::Diamond)`. `Many::some_spaced` sets `min: Some(1)` (lib/game/src/command/parser/mod.rs:310-317) and the `Many` parse impl errors with "expected at least {min} items" when fewer parse (parser/mod.rs:382-391), so `Map` never runs with an empty vec; the fallback is unreachable.
- severity: upheld - nit/quality.
- evidence basis: in-repo.

## F21 Duplicated placings-log block

- verdict: CONFIRMED
- evidence: `lib.rs:754-764` (Take arm) and `lib.rs:777-787` (Sell arm) are character-identical 11-line blocks computing scores/placings and pushing `placings_log`.
- severity: upheld - nit/simplicity.
- evidence basis: in-repo.

## F22 "Rounds remaining" overstated

- verdict: CONFIRMED
- evidence: `render.rs:174`: `remaining_rounds = 3u8.saturating_sub(round_wins[0] + round_wins[1])`. Match end is first-to-2: `is_finished()` at lib.rs:648-650 (`round_wins[p] == 2`), corroborated by DATA_DOCS.md:6 ("First to 2 round wins takes the game") and test `match_finishes_after_two_round_wins` (lib.rs:1114). At 1-0 the display says "There are 2 rounds remaining." but the match can end in 1 more round; at 0-0 it says 3 but can end in 2. The figure is a maximum presented as a fact. (Tied rounds replay without incrementing round_wins, lib.rs:632-636, so >3 total rounds are also possible; saturating_sub only prevents underflow.)
- severity: upheld - nit/correctness (display only).
- evidence basis: in-repo.

## F23 Renderer hides camel count that PubState exposes

- verdict: CONFIRMED
- evidence: `render.rs:40-42` `camel_display` maps 0 -> "no", else -> "some"; used only for the opponent row (render.rs:233-236). But `PubState.camels: [u32; 2]` (lib.rs:186-187) serializes exact counts for both players, and test `pub_state_camels_are_exact` (lib.rs:1316-1321) locks that in. So bots/API consumers see exact opponent camel counts while the human renderer coyly shows "some" - an information-policy inconsistency, whichever direction is intended.
- severity: upheld - nit/consistency.
- evidence basis: in-repo.
