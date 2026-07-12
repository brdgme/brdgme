# splendor-2 Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Go game `brdgme-go/splendor_1` to a Rust crate `rust/game/splendor-2`, register/deploy it, deprecate splendor-1, and update tracking docs.

**Architecture:** Standalone Rust game crate implementing `brdgme_game::Gamer`, following the lost-cities-1 / modern-art-2 template. Domain: 90 development cards across 3 levels, 10 nobles, 5 gem colours + gold, phases Main -> Visit -> Discard -> next player, take/buy/reserve/discard/visit commands.

**Tech Stack:** Rust, brdgme_game, brdgme_markup, brdgme_color, brdgme_cmd, brdgme_fuzz, rand, serde.

## Global Constraints

- Follow `docs/porting/GAME_PORTING.md` and `docs/porting/RENDER_PARITY.md` exactly.
- Porting correctness rule: preserve Go behaviour verbatim, including suspected bugs (see "Quirks to preserve" below). Do not silently fix them.
- **libcost decision (already made):** `brdgme-go/libcost` is ported INLINE into `rust/game/splendor-2/src/cost.rs` (a `Cost` type + only the operations splendor actually uses), not as a shared `rust/lib` crate. It can be extracted to a shared lib later when `seven_wonders` is ported (see `docs/porting/GAME_PORTING_PLAN.md` line 36-39). Port the Go tests for the ported subset 1:1.
  - **Used by splendor_1** (port these): `Cost` (map from resource int -> count), `FromInts`, `Clone`, `Add`, `Sub`, `Inv` (internal to `Sub`), `CanAfford`, `Sum`. Also splendor's own `CanAfford(a, c Cost) bool` in `amount.go` (a *different*, simpler affordability check that also folds in a gold reserve - do not confuse with `libcost.Cost.CanAfford`).
  - **NOT used by splendor_1, drop from the port** (confirmed by `grep -rn '\.Take(\|\.Drop(\|CanAffordPerm\|\.Keys()\|\.IsZero()\|\.Trim()\|\.Ints()\|PosNeg' brdgme-go/splendor_1/` returning zero hits): `Cost.Take`, `Cost.Drop`, `Cost.Keys`, `Cost.IsZero`, `Cost.Trim`, `Cost.Ints`, `Cost.PosNeg`, and all of `libcost/perm.go` (`CanAffordPerm`, `prependToCostArrays`). Note this omission in the port notes (step: tracking docs). Do not port `cost_test.go`'s `TestCost_Take`, `TestCost_Drop`, `TestCost_Ints`, or any of `perm_test.go` - port only `TestCost_Clone`, `TestCost_Add`, `TestCost_Inv`, `TestCost_Sub`, `TestCost_CanAfford`, `TestCost_Sum` (snake_cased).
  - **`Cost.Drop` bug found while reading (not ported, so moot, but recorded for completeness):** `Drop(keys ...int)` builds its exclusion set with `for k := range keys { dm[k] = true }` - this ranges over the *slice index* (0, 1, 2, ...), not `keys[k]`, so it doesn't actually drop the requested resource ids. Irrelevant to the port since `Drop` is unused by splendor, but do not "fix and use" it either - it stays unported.
- Randomness: Go uses time-seeded `rand.New(rand.NewSource(time.Now().UnixNano())).Perm(l)` for both card and noble shuffles. Rust must draw both shuffles from one `rng: GameRng` field seeded via `GameRng::seed_from_u64(seed)` in `start()`, per `docs/authoring/GAME_DEVELOPMENT.md`.
- Placings tie semantics: Go `GenPlacings` is compact-ordinal; Rust `gen_placings` is standard-competition. splendor_1 has no placings test in its Go suite (only `amount_test.go`, `card_test.go`, `noble_test.go` exist - no `game_test.go`), so there is nothing to adapt; write baseline placings tests using Rust's standard-competition semantics directly.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --exclude web --all-targets -- -D warnings` must pass.
- Commit after each task with the trailer: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **The worktree has unrelated uncommitted changes** in `rust/game/roll-through-the-ages-2/src/{command.rs,lib.rs}` and an untracked `docs/superpowers/plans/2026-07-12-roll-through-the-ages-2-port.md`. Never use `git add -A` or `git add .`. Every commit in this plan must `git add` only the specific splendor-related paths it touches.

## Key Go source facts (source of truth: `brdgme-go/splendor_1/`, `brdgme-go/libcost/`)

**Game state** (`game.go`): `Players int`; `Decks [3][]Card`, `Board [3][]Card` (row 0 = level 1, row 1 = level 2, row 2 = level 3); `Nobles []Noble`; `Tokens libcost.Cost` (bank supply); `PlayerBoards []PlayerBoard`; `CurrentPlayer int`; `Phase Phase` (`PhaseMain`/`PhaseVisit`/`PhaseDiscard`, iota 0/1/2); `EndTriggered bool`; `Ended bool`. `PlayerBoard` (`player_board.go`): `Cards []Card` (bought, gives bonuses+prestige), `Reserve []Card` (max 3), `Nobles []Noble` (visited), `Tokens libcost.Cost` (held tokens incl. gold).

**Constants:** `MaxGold = 5`, `MaxTokens = 10`. Players 2-4 (`New` rejects <2 or >4). `MaxGems()`: 2 players -> 4, 3 players -> 5, else (4) -> 7.

**Setup (`New`):** shuffle each level's 40/30/20 cards; board = first 4 of each shuffled level, deck = remainder. Nobles = shuffle all 10 `NobleCards()`, take first `players+1` (2p->3 nobles, 3p->4, 4p->5). `Tokens.Gold = MaxGold`; each of the 5 `Gems` set to `MaxGems()`. Each player gets an empty `PlayerBoard`.

**Card data (`card.go`):** `Resources = [Diamond, Sapphire, Emerald, Ruby, Onyx, Gold, Prestige]` (iota 0-6); `Gems = [Diamond, Sapphire, Emerald, Ruby, Onyx]` (Gold and Prestige excluded). `Card { Resource int, Prestige int, Cost libcost.Cost }`. `Level1Cards()`, `Level2Cards()`, `Level3Cards()` return literal slices. **Transcribe these three functions verbatim into `card.rs` - do not hand-retype values from memory.** After transcribing, verify counts against the Go tests: `Level1Cards()` must have exactly 40 entries, `Level2Cards()` exactly 30, `Level3Cards()` exactly 20 (`card_test.go`: `TestLevel1Cards`/`TestLevel2Cards`/`TestLevel3Cards` assert `Len(..., 40/30/20)`). Port these three as `test_level_1_cards`/`test_level_2_cards`/`test_level_3_cards`.

**Nobles (`noble.go`):** `Noble { Prestige int, Cost libcost.Cost }`. `NobleCards()` returns exactly 10 literal nobles (`noble_test.go`: `TestNobleCards` asserts `Len(..., 10)` -> port as `test_noble_cards`), each `Prestige: 3`, costs are combinations of 3x three-gem (value 3 each) or 2x two-gem (value 4 each) - transcribe verbatim, do not hand-retype.

**Resource strings/colours (`render.go`):** `ResourceColours`: Diamond->Black, Sapphire->Blue, Emerald->Green, Ruby->Red, Onyx->Grey, Gold->Yellow, Prestige->Purple. `ResourceStrings`: "Diamond"/"Sapphire"/"Emerald"/"Ruby"/"Onyx"/"Gold"/"Prestige". `ResourceAbbr`: "Diam"/"Saph"/"Emer"/"Ruby"/"Onyx"/"Gold"/"VP". `GemStrings()` is `ResourceStrings` restricted to the 5 `Gems` keys (used historically by the old text-command parser in the commented-out block; the live parser uses `TokenParser`/`ResourceStrings` directly).

**`amount.go` `CanAfford(a, c libcost.Cost) bool`** (splendor's own affordability check, distinct from `libcost.Cost.CanAfford`): `short := 0`; for each `g, n` in cost `c`, if `a[g] < n` then `short += n - a[g]`; return `a[Gold] - c[Gold] >= short` (i.e. gold in `a` covers the shortfall across all gem types, after subtracting any gold portion of the cost itself - card/noble costs never actually specify `Gold`, so `c[Gold]` is always 0 in practice, but the formula is general). Used both as `PlayerBoard.CanAfford` (via `BuyingPower = Bonuses+Tokens`) and standalone as the noble/board "can afford with bonuses alone" check in `VisitPhase`/render (`CanAfford(pb.Bonuses(), n.Cost)`).

**`Pay(player, amount)` (`game.go`):** validates `pb.CanAfford(amount)` first (already checked by caller too - defence in depth). `offset := pb.Bonuses().Sub(amount)` (bonuses minus cost, per gem, can go negative). For each gem in `Gems`: if `offset[gem] < 0` (bonuses didn't cover it), add the negative offset to both `pb.Tokens[gem]` and back to `g.Tokens[gem]` (i.e. spend `-offset[gem]` normal tokens - this can drive `pb.Tokens[gem]` negative if the player didn't have enough plain tokens); if that drove `pb.Tokens[gem] < 0`, use gold to cover the remainder: `pb.Tokens[Gold] += pb.Tokens[gem]` (adds a negative number, i.e. spends gold), `g.Tokens[gem] += pb.Tokens[gem]` (returns the *shortfall* gems to the bank, since gold substitutes for them - note this returns MORE gems to the bank supply than gold spent numerically balances against, matching the "gold token substitutes for any gem" real-game mechanic), `g.Tokens[Gold] -= pb.Tokens[gem]` (removes that many gold from the bank), then `pb.Tokens[gem] = 0`. Port this arithmetic exactly, including the order of operations (bonuses offset first, tokens second, gold fallback third, per gem in `Gems` order = Diamond, Sapphire, Emerald, Ruby, Onyx).

**Take (`take_command.go`):** `CanTake(player)`: current player and `Phase == PhaseMain`. `Take(player, tokens []int)`:
- Exactly 2 tokens: must be identical (`tokens[0] != tokens[1]` -> error "must take the same type of tokens when taking two"); bank must have `>= 4` of that gem (`g.Tokens[tokens[0]] < 4` -> error "can only take two when there are four or more remaining"); public log `"{player} took {{b}}2 {colour gem name}{{/b}}"`.
- Exactly 3 tokens: pairwise-distinct via circular comparison `tokens[i] != tokens[(i+1)%3]` for each i (catches any duplicate since 3 elements circularly compared) -> error "must take different tokens when taking three"; each must have `g.Tokens[t] > 0` (`== 0` -> error "there aren't enough tokens remaning to take that" [sic - "remaning" typo, preserve in log/error text if displayed, though this is an error message not necessarily rendered - preserve the string exactly if surfaced]); public log `"{player} took {comma list of bold coloured gem names}"` (`brdgme.CommaList`).
- Any other count (0, 1, 4+) -> error "can only take two or three tokens".
- No path allows taking gold via `take` (`TokenParser(includeGold=false)` in `TakeParser`) - gold only comes from `reserve`.
- On success: `amount := Cost{}` built from `tokens`; add to player tokens, subtract from bank; call `g.NextPhase()` (-> visit phase check -> possibly discard phase -> possibly next player) and append those logs.

**Buy (`buy_command.go`):** `CanBuy(player)`: current player and `Phase == PhaseMain`. `Buy(player, row, col)`:
- `row` 0-2 (board): validate `col` in range for `g.Board[row]`; validate `pb.CanAfford(card.Cost)`; `Pay`; append card to `pb.Cards`; refill: if `len(g.Decks[row]) > 0`, replace `g.Board[row][col]` with the top deck card and pop the deck, else remove the slot entirely (`g.Board[row]` shrinks - later cards in that row shift index, and column letters in the next `LocParser` call shift accordingly since letters are assigned by current slice position, not a stable id). Public log `"{player} bought {card} from the board"`.
- `row` 3 (reserve): validate `col` in range for `pb.Reserve`; validate afford; `Pay`; append to `pb.Cards`; remove from `pb.Reserve` (slice delete, shifting later reserve indices/letters down). Public log `"{player} bought {card} from their reserve"`.
- any other row -> error "that is not a valid row".
- `g.Pay`'s returned error is discarded (`_ = g.Pay(...)`) since affordability was already validated - Rust may still propagate an error from the equivalent call but it must never actually trigger given the prior `CanAfford` check (treat as an invariant, not a fallible path a real command can hit).
- On success: `g.NextPhase()` appended.

**Reserve (`reserve_command.go`):** `CanReserve(player)`: current player, `Phase == PhaseMain`, `len(pb.Reserve) < 3`. `Reserve(player, row, col)`: row must be 0-2 (board only - **cannot reserve from your own reserve or reserve blind from a deck**, only face-up board cards); col must be valid for `g.Board[row]`. Public log `"{player} reserved {card}"` (**the reserved card's identity is broadcast publicly in the log at reservation time**, regardless of who reserves it). Card moves to `pb.Reserve`. If `g.Tokens[Gold] > 0`, player gains 1 gold and bank loses 1 (no gold if bank is out - **no compensating token substitute, the gold is just skipped**, this is real-game-correct). Board slot refilled from deck or removed exactly as in `Buy`. `g.NextPhase()` appended.

**Discard (`discard_command.go`):** `CanDiscard(player)`: current player and `Phase == PhaseDiscard`. `Discard(player, tokens []int)`: tokens list must be non-empty; build `tCost := libcost.FromInts(tokens)`; validate `pb.Tokens.CanAfford(tCost)` (this is `libcost.Cost.CanAfford`, i.e. player must currently hold at least that many of each named token - **note this uses the raw `libcost.Cost.CanAfford`, not splendor's own `amount.go` `CanAfford`, and does not involve gold-shortfall logic at all**, it's a plain per-key `>=` check); subtract from player tokens, add to bank; public log `"{player} discarded {comma list of bold coloured names}"`; if `pb.Tokens.Sum() <= MaxTokens` after discarding, call `g.NextPhase()` and append (else remain in `PhaseDiscard`, player must discard more). Discard allows discarding gold (`TokensParser(includeGold=true)`).

**Visit (`visit_command.go`):** `CanVisit(player)`: current player and `Phase == PhaseVisit`. `Visit(player, noble int)`: `noble` must be a valid index into `g.Nobles` (0-based after the parser's `-1`). **No affordability re-check inside `Visit` itself** - it unconditionally lets the player visit *any* noble by index, appends that noble to `pb.Nobles`, public log `"{player} was visited by {noble}"`, removes the noble from `g.Nobles`, then `g.NextPhase()`.

**Visit-phase auto-advance (`VisitPhase` in `game.go`):** sets `Phase = PhaseVisit`; computes `canVisit []int` = indices of `g.Nobles` where `CanAfford(pb.Bonuses(), noble.Cost)` (bonuses-only, no tokens) is true for the *current* player. If 0 candidates: immediately advance (`NextPhase()` -> discard phase check). If exactly 1 candidate: auto-visit that noble via `g.Visit(...)` directly (bypassing `CanVisit`'s phase gate is fine since `Phase` was already set to `PhaseVisit` just above) and return its logs, no player choice offered. If 2+ candidates: return nil (stay in `PhaseVisit`, wait for player's `visit` command) - **and the `VisitParser` built during this wait allows choosing any of `len(g.Nobles)` nobles by number, not just the affordable ones in `canVisit`** (see quirk below).

**Discard-phase auto-advance (`DiscardPhase`):** sets `Phase = PhaseDiscard`; if current player's token sum `<= MaxTokens` (10), immediately advance via `NextPhase()`; else wait for `discard` commands.

**Phase chaining (`NextPhase`):** `PhaseMain -> VisitPhase()`, `PhaseVisit -> DiscardPhase()`, `PhaseDiscard -> NextPlayer()`. Every action (`Take`/`Buy`/`Reserve`/`Discard`/`Visit`) that completes successfully calls `g.NextPhase()` once and appends its logs (chaining through auto-advanced phases in the same call as needed).

**End of turn (`NextPlayer`):** `logs := g.CheckEndTriggered()` (checked BEFORE incrementing turn, scans **all** players' `Prestige() >= 15`, sets `g.EndTriggered = true` and returns a public bold log "The end of the game has been triggered" the *first* time any player crosses the threshold - only fires once, `if g.EndTriggered { return nil }` guards re-firing); `g.CurrentPlayer = (g.CurrentPlayer + 1) % g.Players`; if `g.EndTriggered && g.CurrentPlayer == 0`, set `g.Ended = true` (game ends only once play has wrapped back around to player 0, giving every player an equal number of turns in the final round - **note this means a 3-or-4-player game's last round is not just "the round the trigger fired in" but continues until player 0's next turn**); else call `g.MainPhase()` (sets `Phase = PhaseMain` for the new current player - note this is *not* called when the game ends, so `Phase` is left at `PhaseDiscard` from the ending move).

**Scoring/placings:** `Points()` returns each player's `PlayerBoard.Prestige()` (sum of owned card prestige + visited noble prestige) as `float32`. `Placings()` metrics per player: `[Prestige(), len(Cards)]` (card count is the tiebreaker, more cards = better placement, no further tiebreakers). Go has no placings test for this game to check tie semantics against - use Rust `gen_placings` standard-competition semantics directly for baseline tests (see Global Constraints).

**Loc parsing (`command.go`):** `LocParser(player)` builds an `Enum` from: for each board row/col currently populated, name `"{'A'+col}{row+1}"` (e.g. row 0 col 0 -> "A1", row 2 col 3 -> "D3") -> `ParsedLoc{Row: row, Col: col}`; then for each of the player's own reserved cards by index, name `"{'A'+col}4"` -> `ParsedLoc{Row: 3, Col: col}`. **Column letters are positional, not stable ids** - they shift whenever a card leaves an earlier column (board refill replaces in place so board columns stay stable *only* when the deck still has cards for that row; once a row's deck is empty, buying/reserving from it deletes the slot and every later card in that row shifts down a letter). Same for reserve: buying reserve slot 0 shifts what was slot 1 down to letter "A4".

**Token parsing:** `TokenParser(includeGold)`: enum of `Gems` (+`Gold` if `includeGold`) keyed by `ResourceStrings` name (e.g. "Diamond", "Gold"). `TokensParser(includeGold)`: one-or-more (`Min: 1`) space-delimited `TokenParser` values collected into `[]int`. `take` uses `includeGold=false`; `discard` uses `includeGold=true`.

**Commands (`command.go`):** `CommandParser` builds `OneOf` from whichever of `CanBuy`/`CanDiscard`/`CanReserve`/`CanTake`/`CanVisit` are true for `player`, in that fixed order (buy, discard, reserve, take, visit); returns `nil` (-> "not expecting any commands at the moment" error) if none apply. All five commands set **`CanUndo: false` in every `CommandResponse`** (verbatim from `BuyCommand`/`DiscardCommand`/`ReserveCommand`/`TakeCommand`/`VisitCommand` in `command.go` - none of the five actions ever returns `CanUndo: true`; unlike some other ports there is no deterministic no-info-reveal action here to make undoable, since board refills, gold reservation timing, and choice-of-noble/discard-tokens are all treated the same for undo purposes by the source. Preserve `can_undo: false` for all five in the Rust port).

**Rendering (`render.go`):** `PubRender()` calls `PlayerRender(-1)`; both share one function keyed on `pNum >= 0`. Sections in order, each a `render.Table` (note: Go's `Table(rows, rowSpacing, colSpacing)` inserts literal blank spacer cells of `colSpacing` spaces between every column when `colSpacing > 0`, and blank spacer rows of `rowSpacing-1` newlines between every row when `rowSpacing > 0`; every table in this file uses `rowSpacing = 0`, so no row spacers are needed, but column spacers must be inserted by hand in Rust - see `docs/porting/RENDER_PARITY.md`):
1. **Nobles table** (`Table(rows, 0, 2)`, colSpacing 2): header row = blank cell + one centred grey index (1-based) per noble; content row = grey label `"Nobles ({{b}}3{{/b}} each)"` (3 rendered in Prestige colour/bold) + one `RenderAmount(noble.Cost)` cell per noble.
2. **Board table** (`Table(rows, 0, 3)`, colSpacing 3): header row = blank cell + one bold grey centred column letter per column, `longestRow` = max(over the 3 board rows' lengths, and if a player view, also `len(pb.Reserve)`). Then for each level 0-2: an "upper" row (grey `"Level {{b}}N{{/b}}"` label cell, then per card: if player view and `CanAfford(bonuses, cost)` prefix bold green `"X "`, else if player view and `pb.CanAfford(cost)` prefix bold yellow `"X "`, then `RenderCardBonusVP` = bold `"{abbr resource in colour}[ {prestige in Prestige colour}]"`) and a "lower" row (blank label cell, then `RenderAmount(cost)` centred per card) followed by a blank spacer row `[]render.Cell{}` (an empty row - zero cells - appended directly, not via rowSpacing). After the 3 levels: a "Level 4 / Reserved" pair of rows (no trailing blank row after this one) built the same way over `pb.Reserve` **only when `pNum >= 0`** (pub render, `pNum == -1`, shows the "Level 4"/"Reserved" header row and label row but zero card columns - reserved card identities are never shown in the pub/spectator render, and are shown to every OTHER player as an empty "Reserved" row too - only the owning player's own `PlayerRender` populates their reserve cards; opponents' reserve *counts* appear elsewhere, see Player table below).
3. **Tokens table** (`Table(rows, 0, 3)`, colSpacing 3): header row (blank + bold coloured abbr per gem, then Gold); if player view, "You have" row (`bonuses[gem]+tokens[gem]`, bold, centred) and a grey `"(bonus+token)"` sub-row reading literally `"(card+token)"` per gem except Gold (Gold's desc cell is blank - the format string is only applied `if gem != Gold`); "Tokens left" bank-supply row always present.
4. **Player table** (`Table(rows, 0, 2)`, colSpacing 2): header = blank + bold coloured abbr per gem (Diamond/Sapphire/Emerald/Ruby/Onyx) + bold coloured Gold abbr + bold "Tok" + bold cyan "Res" + bold coloured Prestige("VP") abbr + bold "Dev"; one row per player (`render.Player(p)` name cell), bold-if-`p == pNum` for every stat cell: per gem `"{bonus}+{token}"`, then plain gold token count, `Tokens.Sum()`, `len(Reserve)`, `Prestige()`, `len(Cards)`.
- Section separators: two blank lines after nobles (`"\n\n"`), three blank lines after board (`"\n\n\n"`), three blank lines after tokens (`"\n\n\n"`); no trailing separator after the player table (last section).
- `RenderCard(c)`: `"{RenderCardBonusVP} ({RenderAmount cost})"`. `RenderNoble(n)`: bold `"{prestige in Prestige colour} ({RenderAmount cost})"`. `RenderAmount(a)`: for each resource in fixed `Resources` order, if `a[r] > 0` emit bold coloured number, join with grey `"-"`.
- **`PubState`/`PlayerState` in Go both return `nil`** (`game.go`: `func (g *Game) PlayerState(player int) interface{} { return nil }`, same for `PubState`) - render reads `*Game` fields directly, not a serialized state struct. Per `GAME_PORTING.md` step 6, Rust's `PubState`/`PlayerState` must be modelled on what `render.go` actually shows, not on this empty stub: `PubState` needs the board (all 3 rows, `Card` + refill-availability irrelevant to render but deck *counts* aren't shown either so decks can be entirely omitted from `PubState`/`PlayerState` - render never displays deck size), nobles, bank tokens, and *for every player* their bonuses-derived-from-cards, tokens, prestige, card count, reserve count (but NOT reserve card contents for anyone but the viewing player) and visited-noble list/prestige. `PlayerState` = `{ public: PubState, player: usize, reserve: Vec<Card> }` (the viewing player's own full reserve card list, which is the one piece of information `PubState` must NOT carry - it is the only hidden information in this game, since deck order is not separately observable through render at all, board contents are always fully visible face-up, and everyone's `Cards`/`Nobles`/bonuses/prestige/token counts are fully public).

## Quirks to preserve (verbatim, do not silently fix)

1. **Visit-command has no affordability re-check.** When 2+ nobles are affordable (`VisitPhase`'s `canVisit` has length >= 2), the game waits for a manual `visit N` command, but `VisitParser`/`Visit()` accept **any** of the `len(g.Nobles)` nobles by 1-based index - not just the ones in `canVisit`. A player can visit (and gain points from) a noble whose cost they cannot actually meet. Port this exactly: the Rust `Command::Visit` variant's parser range is `1..=nobles.len()`, and the `visit` action performs no affordability check, matching Go.
2. **Reserved card identity is publicly logged, then privately rendered.** `Reserve`'s log message (`"{player} reserved {full card name+cost}"`) is `brdgme.NewPublicLog` - broadcast to everyone at the moment of reservation - yet the subsequent board/player render (`render.go`) never shows a non-owner's reserve card contents, only the owning player's own reserve and everyone's reserve *count* (via the Player table's "Res" column). The information is technically already public via the log, but the persistent render still redacts it for other players. Preserve both halves: public log with full card detail, `PubState`/`PlayerState` (for other players) without reserve card contents.
3. **`NextPlayer` end-trigger-then-wrap semantics.** `CheckEndTriggered` scans all players and sets `EndTriggered` (once, ever) the moment *any* player reaches 15+ prestige, but `Ended` is only set on the specific `NextPlayer` call where the turn wraps back to player 0 - meaning every player gets an equal number of turns in the final round regardless of whose turn triggered the end, and `Phase` is left however it was on the game-ending move (`MainPhase()` is skipped when `Ended` becomes true).
4. **Board/reserve column letters are positional, not stable ids.** Once a row's deck is exhausted, buying or reserving a card deletes that slot outright rather than leaving a gap, so every later card's letter in that row (and every later reserve card's letter) shifts down by one. `A1`/`B2`/etc. must be recomputed fresh from current board/reserve state on every parse, never cached.
5. **`Pay`'s gold-fallback step returns more bank gems than it consumes in gold** by design (mirrors the "gold is a wildcard for any single gem" rule) - the arithmetic in `game.go`'s `Pay` (offset via bonuses, then tokens, then gold) must be ported with the exact per-gem order (`Gems` = Diamond, Sapphire, Emerald, Ruby, Onyx) since it's iterated as a fixed slice, not a map (whose iteration order would be nondeterministic in Go, but here `for _, gem := range Gems` fixes it).
6. **`libcost.Cost.Drop` has an index/value bug** (`for k := range keys` uses the slice index, not `keys[k]`) - noted for completeness; irrelevant to the port since `Drop` is never called by splendor and will not be ported at all.
7. **Take-two requires bank `>= 4`, not `>= 2`,** of that gem (`g.Tokens[tokens[0]] < 4` is the rejection condition) - this is intentional (real Splendor rule: must leave at least 2 in the supply after taking 2), not a bug, but easy to mis-port as `< 2`.

## Baseline test suite (per GAME_PORTING.md step 8 - Go suite is 3 files, ~53 lines, no game-level tests)

Go tests to port 1:1 (snake_cased names): `test_can_afford` (amount_test.go, both assert cases), `test_level_1_cards`/`test_level_2_cards`/`test_level_3_cards` (card_test.go), `test_noble_cards` (noble_test.go), plus from the ported cost subset: `test_cost_clone`, `test_cost_add`, `test_cost_inv`, `test_cost_sub`, `test_cost_can_afford`, `test_cost_sum`.

Add at minimum (no existing Go game-level test file to port from):
- `start` / `New`: rejects 1 and 5 players; accepts 2-4; board is 4 cards per level, deck is 36/26/16 remaining per level; nobles count = players+1; bank tokens = `MaxGems()` per gem (4/5/7 for 2/3/4p) + 5 gold; every player board starts empty; `Phase == Main`, `CurrentPlayer == 0`.
- Token pools per player count: assert `MaxGems()` mapping (2->4, 3->5, 4->7) end to end via `start`.
- `take` valid: 2 identical (bank has >=4), 3 distinct (all >0 in bank); invalid: 2 different, 3 with a repeat, count 0/1/4/5, taking a gem the bank has 0/1/2/3 of (for the 2-same and 3-distinct thresholds respectively), taking gold via `take` (should be rejected/unparseable since `TakeParser` excludes gold).
- `buy`: paying using only bonuses, only tokens, tokens+gold fallback (construct a player board with cards for bonuses and limited tokens to force the gold path and assert bank/player token deltas match the `Pay` arithmetic in "Key Go source facts"); buying from board (all 3 rows) with and without deck refill available (row's deck empty -> slot removed, letters shift); buying from own reserve; buying unaffordable card errors; buying invalid loc errors.
- `reserve`: reserving fills a reserve slot and grants 1 gold if bank has gold, no gold if bank gold is 0; reserving at 3 existing reserved cards is rejected (`CanReserve` false); reserving from row 3 (i.e. attempting to reserve your own reserve slot) is rejected/unparseable; reserved card is logged publicly with full detail (assert on `Vec<Log>`); reserved-card visibility test per `GAME_DEVELOPMENT.md`'s "Tests" section - after another player reserves, assert non-owner's `PubState`/`PlayerState` do not contain that card's identity, only an incremented reserve count.
- `discard`: enforced only when total tokens > `MaxTokens` (10); discarding down to exactly 10 in one command re-enters Main phase for the *next* player (i.e. `NextPhase` chain resumes); discarding insufficient tokens leaves `Phase == Discard` and rejects further non-discard commands; discarding tokens the player doesn't hold is rejected; discarding gold is allowed.
- `visit`: 0 affordable nobles -> auto-skips to discard/main without a `visit` command being available; exactly 1 affordable noble -> auto-visits with a log, no `visit` command offered; 2+ affordable -> `visit` command is available and accepts any noble index 1..=len(nobles) including ones the player can't actually afford (quirk 1) - assert both the happy affordable-choice path and the unaffordable-choice-still-succeeds path.
- End trigger and final round: a player reaching 15+ prestige sets `EndTriggered` and logs "The end of the game has been triggered" exactly once; game does not end immediately, continues until the turn wraps to player 0 (`Ended == true` only then); a second player crossing 15 after trigger does not re-log.
- `placings`: `[prestige, card_count]` metrics; construct a tie in prestige broken by card count; construct a true tie (both prestige and card count equal) and assert Rust's standard-competition placings (e.g. two tied at rank 1 produce `[1, 1, 3]`, not compact-ordinal `[1, 1, 2]`).
- `pub_state` capture: after a `reserve`, assert pub state has correct reserve *count* per player but no reserved-card content for any player except (in that player's own `player_state`) themselves.
- Command after finished errors; command from the wrong player errors (both for `command()` dispatch and for `command_spec` returning `None`/empty for non-current players).
- Parser regression cases from the Go source: `take Diamond Diamond` (2 same, valid), `take Diamond Sapphire` (2 different, rejected), `take Diamond Sapphire Ruby` (3 distinct, valid), `buy A1`, `buy A4` (reserve slot 0), `reserve B2`, `discard Gold`, `visit 1`; also confirm case/format handling matches the `Enum`/`Token` parser combinators (e.g. token names come from `ResourceStrings`, loc names are exactly `{Letter}{Row1-4}` with no separator).

---

### Task 1: Crate skeleton + cost module + domain types

**Files:**
- Create: `rust/game/splendor-2/Cargo.toml` (copy modern-art-2's, rename package to `splendor-2`, bins `splendor_2_{cli,http,repl,fuzz}`)
- Create: `rust/game/splendor-2/src/cost.rs` (ported `Cost` subset + splendor's own `amount.go` `can_afford` function + ported libcost tests, see Global Constraints for exact scope)
- Create: `rust/game/splendor-2/src/card.rs` (`Resource` enum incl. Gold/Prestige-as-colour-key, `Gems` const list, `Card` struct, `level_1_cards()`/`level_2_cards()`/`level_3_cards()` transcribed verbatim from `card.go`, `Noble` struct + `noble_cards()` transcribed verbatim from `noble.go`)
- Create: `rust/game/splendor-2/src/player_board.rs` (`PlayerBoard` struct: cards, reserve, nobles, tokens; `bonuses()`, `buying_power()`, `can_afford()`, `prestige()`)
- Create: `rust/game/splendor-2/src/bin/splendor_2_{cli,http,repl,fuzz}.rs` (copy modern-art-2 stubs, rename)
- Create: `rust/game/splendor-2/tests/contract.rs` (`assert_gamer_contract::<Game>()` - stub `Game` from Task 2 needed first; may defer this file's creation to Task 2 if `Game` doesn't exist yet, but Cargo.toml/workspace member registration happens now)
- Create: `rust/game/splendor-2/RULES.md` (placeholder single line; full content in Task 4)
- Modify: `rust/Cargo.toml` (add `game/splendor-2` to workspace `members`)

**Interfaces:**
- Produces: `splendor_2::cost::Cost` (+ used ops), `splendor_2::card::{Resource, Card, Noble, level_1_cards, level_2_cards, level_3_cards, noble_cards}`, `splendor_2::player_board::PlayerBoard`.

- [x] Step 1: Read reference crate `rust/game/modern-art-2` (all of `src/`, `Cargo.toml`, `tests/contract.rs`) in full for template details (bins, dev-deps, test layout).
- [x] Step 2: Write failing tests first (inline `#[cfg(test)]` modules): ported cost tests (`test_cost_clone`, `test_cost_add`, `test_cost_inv`, `test_cost_sub`, `test_cost_can_afford`, `test_cost_sum`), `test_can_afford` (amount.go's function, both cases from `amount_test.go`), `test_level_1_cards`/`test_level_2_cards`/`test_level_3_cards` (assert lengths 40/30/20), `test_noble_cards` (assert length 10, plus assert every noble's `prestige == 3`).
- [x] Step 3: Implement `cost.rs`, `card.rs` (transcribing Go card/noble literals verbatim - re-open `brdgme-go/splendor_1/card.go` and `noble.go` side by side while transcribing, do not rely on earlier reads from memory), `player_board.rs` until `cargo test --package splendor-2` passes.
- [x] Step 4: `cargo build --package splendor-2` builds (bins may be trivial stubs pending Task 2's `Game`; if `Game` doesn't exist yet, stub bins to not reference it, or defer bin creation to Task 2 - implementer's choice, note which in the commit).
- [x] Step 5: Commit `feat(splendor-2): crate skeleton, cost module, card and noble data` touching only `rust/game/splendor-2/` and `rust/Cargo.toml`.

### Task 2: Game engine + commands

**Files:**
- Create: `rust/game/splendor-2/src/lib.rs` (`Game` struct, `Phase` enum, `Gamer` impl: `start`, `player_count`, `player_counts`, `command`, `command_spec`, `status`, `points`, `pub_state`, `player_state`, `rules`)
- Create: `rust/game/splendor-2/src/command.rs` (`Command` enum: `Buy(ParsedLoc)`, `Discard(Vec<Resource>)`, `Reserve(ParsedLoc)`, `Take(Vec<Resource>)`, `Visit(usize)`; `command_parser`, `loc_parser`, `token_parser`, `tokens_parser`)
- Modify/Create: `rust/game/splendor-2/tests/contract.rs`
- Modify: `rust/game/splendor-2/src/bin/*` if deferred from Task 1

**Interfaces:**
- Consumes: `cost.rs`, `card.rs`, `player_board.rs` from Task 1.
- Produces: `splendor_2::Game` implementing `brdgme_game::Gamer`.

- [ ] Step 1: Read `brdgme-go/splendor_1/{game.go,command.go,take_command.go,buy_command.go,reserve_command.go,discard_command.go,visit_command.go}` again in full before implementing (re-verify against "Key Go source facts" above rather than relying purely on this document).
- [ ] Step 2: Write failing tests first per the "Baseline test suite" section above, plus 1:1 nothing-else-to-port (the 3 Go test files are fully covered in Task 1; this task's tests are entirely the baseline suite since `splendor_1` has no `game_test.go`/`command_test.go`).
- [ ] Step 3: Implement `command.rs` parsers (buy/discard/reserve/take/visit, `LocParser` computed fresh from current board+reserve state, `TokenParser`/`TokensParser` with `include_gold` flag) and `lib.rs` (phases, `pay`, `take`/`buy`/`reserve`/`discard`/`visit` actions, `next_phase`/`visit_phase`/`discard_phase`/`next_player`/`check_end_triggered`, `placings`). Use the borrow-order pattern from GAME_PORTING.md for `command()`. Preserve all "Quirks to preserve" verbatim. All five `CommandResponse`s use `can_undo: false`.
- [ ] Step 4: `cargo test --package splendor-2` passes; `cargo build --package splendor-2` builds all 4 bins; `tests/contract.rs` passes.
- [ ] Step 5: Commit `feat(splendor-2): game engine, phases, and commands` touching only `rust/game/splendor-2/`.

### Task 3: Render + render parity

**Files:**
- Create: `rust/game/splendor-2/src/render.rs` (`PubState`, `PlayerState`, `Renderer` impls porting `render.go`'s nobles/board/tokens/player-table sections with hand-inserted spacer cells)
- Modify: `rust/game/splendor-2/src/lib.rs` (wire `pub_state`/`player_state` to use the new types if not already correct from Task 2)

**Interfaces:**
- Consumes: `Game` state from Task 2.

- [ ] Step 1: Re-read `brdgme-go/splendor_1/render.go` in full and port each section faithfully (spacer cells matching colSpacing 2/3/3/2 per table as documented in "Key Go source facts"; blank spacer row after each of the 3 board levels; the "Level 4/Reserved" pair with empty columns for pub/non-owner views; the "(card+token)"-labelled sub-row with blank Gold cell; bold-if-viewing-player in the Player table).
- [ ] Step 2: Build Go CLI (`go build -o /tmp/render-parity/splendor_1_go ./brdgme-go/splendor_1/cmd`), Rust CLI (`cargo build --package splendor-2 --bin splendor_2_cli`), and `render_plain` (`cargo build --package brdgme_render_plain`); compare pub render and every player render at `New` (2p and 4p) and after representative mid-game commands (take, buy, reserve, discard trigger, visit with 0/1/2+ affordable nobles) using `scripts/render-compare/render.sh`, identical names both sides. Structural comparison only (RNG differs - card/noble contents will differ, compare wording/spacing/alignment/section order/labels).
- [ ] Step 3: Fix discrepancies until wording/spacing/alignment/ordering match, paying special attention to the reserve-card-hidden-from-non-owners behaviour (opponent's Reserved row must render as present-but-empty, matching Go, not omitted entirely). Record the side-by-side outputs in the task report.
- [ ] Step 4: `cargo test --package splendor-2` still green. Commit `feat(splendor-2): render parity with Go` touching only `rust/game/splendor-2/`.

### Task 4: RULES.md

**Files:**
- Modify: `rust/game/splendor-2/RULES.md`

- [ ] Step 1: Read `docs/authoring/RULES_AUTHORING.md` in full and the splendor source; write RULES.md per required sections (Overview, Components incl. 90 cards/10 nobles/token supply table by player count, Turn Structure with inline `take`/`buy`/`reserve`/`discard`/`visit` commands and the visit auto-advance behaviour, Scoring with a worked prestige example, Game End [15-prestige trigger + finish-the-round-to-player-0 wrap], Winning [prestige then card count tiebreak], Reading the Display with a real render captured from the Rust CLI in a ```brdgme block, Commands table, Strategy Tips section).
- [ ] Step 2: Verify command syntax against `command.rs` and scoring/end-of-game against `lib.rs`. Commit `docs(splendor-2): add RULES.md` touching only `rust/game/splendor-2/RULES.md`.

### Task 5: Registration, deprecation, fuzz, lint

**Files:**
- Modify: `rust/Dockerfile` (final stage `splendor-2` copying `splendor_2_http`, mirroring an existing Track B stage)
- Modify: `docker-bake.hcl` (add `splendor-2` to `tgt` array)
- Modify: `Tiltfile` (add `"splendor-2"` to Rust games list)
- Create: `k8s/base/game/splendor-2/{deployment.yaml,service.yaml,game-version.yaml,kustomization.yaml}` (mirror an existing Track B game's manifests, e.g. `k8s/base/game/splendor-1/` for naming/display text conventions; GameVersion display name "Splendor")
- Modify: `k8s/base/game/kustomization.yaml` (add dir)
- Modify: `k8s/prod/app/kustomization.yaml` (add `ghcr.io/brdgme/brdgme/splendor-2` image override)
- Modify: `k8s/base/game/splendor-1/game-version.yaml` (add `isDeprecated: true`)

- [ ] Step 1: Make all registration edits, mirroring an existing deployed Track B game's entries exactly (`k8s/base/game/splendor-1` already exists in-repo - read it first for the display name/weight conventions to reuse).
- [ ] Step 2: Run fuzzer `cargo run --release --package splendor-2 --bin splendor_2_fuzz` for ~2 minutes; zero panics required.
- [ ] Step 3: `cargo fmt --all -- --check`, `cargo clippy --workspace --exclude web --all-targets -- -D warnings`, `cargo test --package splendor-2` all clean. Record test count via `cargo test --package splendor-2 -- --list --format terse | rg ': test$' | wc -l`.
- [ ] Step 4: Commit `feat(splendor-2): register and deploy, deprecate splendor-1` touching only `rust/Dockerfile`, `docker-bake.hcl`, `Tiltfile`, `k8s/base/game/splendor-2/`, `k8s/base/game/kustomization.yaml`, `k8s/prod/app/kustomization.yaml`, `k8s/base/game/splendor-1/game-version.yaml`.

### Task 6: Tracking docs

**Files:**
- Modify: `docs/porting/GAME_PORTING_PLAN.md` (add "Done (Track B, <date>): splendor-2 ported..." entry with test counts, fuzz numbers, port notes incl. the libcost-inline decision and the dropped `Cost.Take`/`Drop`/`Keys`/`IsZero`/`Trim`/`Ints`/`PosNeg`/`perm.go` functions, matching prior entries' style; also update the `splendor` row in the Track B table (currently `| splendor | 2,262 (53) | cost | Needs cost module |`) to reflect completion)
- Modify: `docs/BACKLOG.md` (move/mark splendor port done if listed there)

- [ ] Step 1: Write the done entry and port-specific notes (quirks preserved, libcost subset ported, placings semantics used, any decisions made without user input).
- [ ] Step 2: Commit `docs: mark splendor-2 port done` touching only `docs/porting/GAME_PORTING_PLAN.md` and `docs/BACKLOG.md`.

---

## Self-review notes

- All Go test cases across `amount_test.go`, `card_test.go`, `noble_test.go`, and the ported `libcost` subset's `cost_test.go` cases are covered in Task 1; the (empty) game-level Go test gap is filled by the baseline suite in Task 2.
- libcost scope decision and dropped functions are recorded in Global Constraints and repeated in Task 6's tracking-doc update so the omission is visible in the PR trail.
- Placings tie semantics: no Go test exists to adapt, baseline tests use Rust standard-competition directly, documented in Global Constraints.
- Reserved-card visibility (the one piece of hidden information in this game) is called out in "Key Go source facts" point 2 of render.go, in Quirks-to-preserve #2, in the baseline test suite, and in Task 3's render-parity step - deliberately repeated since it is the single easiest thing to get wrong (`PubState` leaking it, or render hiding it from the owner too).
- Render spacer-cell trap (colSpacing 2/3/3/2 across the four tables, zero rowSpacing throughout) called out in Key facts and Task 3.
