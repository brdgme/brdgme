# Verification: games-batch-f (zombie-dice-2, battleship-2) — Worker W1, 2026-07-24

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust
Go: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go

### F1 Cup draw order leaked to all players in PubState — CONFIRMED
- evidence: PubState field `pub cup: Vec<Dice>` with doc "Dice remaining in the cup, in draw order" (zombie-dice-2/src/lib.rs:193-194); `pub_state()` clones it verbatim: `cup: self.cup.clone()` (lib.rs:443). Order is meaningful: the cup is shuffled only at turn start (`start_turn` -> `shake_cup`, lib.rs:256-257) and on refill (lib.rs:249), and draws come off the front via `self.cup.drain(..n)` (lib.rs:251), so the vector head IS the next draw. Between rolls within a turn the order is stable and fully predictive. Go returned nil: `func (g *Game) PubState() interface{} { return nil }` (brdgme-go/zombie_dice_1/game.go:55-57), so the leak is new in the port. DATA_DOCS.md:18 says "Zombie Dice has no hidden information per player" — inaccurate as claimed. The renderer only shows counts (`render_cup`, render.rs:51-73), so the leak is JSON-contract-only, matching the finding.
- severity: agree, major. Real hidden-information leak materially affecting roll/keep decisions for any API/bot consumer; not data-loss/security of the platform, so not critical. Consistent with the batch's other major (for-sale-2 bids leak, also rated major).
- recommendation-check: valid. Either per-colour counts (renderer already needs only counts) or sorting/canonicalizing the vec in `pub_state()` destroys the order information; DATA_DOCS.md fix is a doc edit. No side effects: PubState is serialize-only output, nothing reads `cup` order from PubState.

### F2 Cup refill returns shotgun dice too (Go quirk) — CONFIRMED (external basis for the official-rules claim)
- evidence: refill branch returns ALL kept dice: `let returned: Vec<Dice> = self.kept.iter().map(|dr| dr.dice).collect(); self.cup.extend(returned); self.kept = vec![];` (lib.rs:246-248) — `kept` contains both Brain and Shotgun results (lib.rs:326-333). Go identical: `for _, d := range g.Kept { g.Cup = append(g.Cup, d.Dice) }` (zombie_dice_1/game.go:108-110). RULES.md:31 documents it: "If the cup runs dry, all your set-aside dice (brains and shotguns) go back in". The claim that official SJG rules return only brains is external-rulebook knowledge not in the repo; noted as external basis, not verified from repo contents.
- severity: agree, minor. Ported behaviour, documented, cross-reference for the parity decision list.
- recommendation-check: valid. "Decide deliberately" with either option is coherent; filtering `Face::Brain` from `kept` is straightforward and RULES.md already describes the current behaviour if kept.

### F3 Rolloff state not exposed in PubState / render — CONFIRMED
- evidence: `Game.roll_off_players` exists (lib.rs:173) but PubState (lib.rs:185-207) has no corresponding field and `pub_state()` (lib.rs:438-455) copies nothing from it; render.rs:92-145 renders only the status table and scores, no rolloff indication. The only signal is the one-time "tie breaker round!" log (lib.rs:279-285). Non-participants are silently skipped via `should_skip_in_rolloff` recursion (lib.rs:288-291). Go had no pub state at all (game.go:55-57), so this is new-surface incompleteness, not a regression.
- severity: agree, minor. Client-visible completeness gap, no game-logic error.
- recommendation-check: valid. Adding `roll_off_players: Vec<usize>` to PubState, populating and rendering it is additive and leaks nothing (rolloff membership is public info from the log).

### F4 Panic paths on inconsistent deserialized state — CONFIRMED
- evidence: `self.cup.drain(..n)` panics if `cup.len() < n` still after refill (lib.rs:251); `self.scores[self.current_turn]` (lib.rs:368, :374); `leaders()` indexes `self.scores[p]` for `p in 0..self.players` (lib.rs:384-390); `(self.current_turn + 1) % self.players` panics on `players == 0` (lib.rs:267); render.rs:132-139 indexes `self.scores[p]` for `p in 0..self.players`. `Game` derives `Deserialize` with all-pub fields (lib.rs:163-183), no validation, so mismatched `scores.len()`/`players`/dice-partition in stored state panics. Not reachable from commands (parser only yields Roll/Keep). Go panics identically on `g.Cup[:n]` (game.go:116). Matches the batch's unit-wide systemic note.
- severity: agree, minor. Crafted-stored-state only, systemic fix tracked at requester/serde layer.
- recommendation-check: valid. Clamping `n` to `cup.len()` and validating lengths on load are safe; "otherwise acceptable" is a reasonable default given the systemic note.

### F5 Unbounded recursion chain on repeated busts — CONFIRMED
- evidence: bust path: `roll()` on `round_shotguns >= BUST_SHOTGUN_COUNT` calls `self.next_player()` (lib.rs:340-347); `next_player()` calls `self.start_turn()` (lib.rs:291); `start_turn()` ends with `self.roll()` (lib.rs:262). A first roll of a fresh turn rolls 3 dice and can produce 3 shotguns (BUST_SHOTGUN_COUNT = 3), so consecutive first-roll busts chain one stack frame set per bust. Also `next_player()` self-recurses per skipped rolloff player (lib.rs:288-289), bounded by player count. RNG is server-side (`GameRng` in state, lib.rs:182), not adversarially steerable. Probability of deep chains is negligible (3 shotguns on 3 dice each turn).
- severity: agree, nit. Theoretical.
- recommendation-check: valid. "None required; convert to a loop if touched" is appropriate.

### F6 Rolloff tie announcement re-logged every wrap — CONFIRMED
- evidence: `next_player()` runs the tie check on every wrap to player 0 (`if self.current_turn == 0`, lib.rs:268); while the top scores remain tied >= WIN_SCORE and `leaders.len() > 1`, it unconditionally reassigns `self.roll_off_players = leaders.clone()` and re-pushes the identical "tie breaker round!" log (lib.rs:276-285). No guard on `roll_off_players` already being non-empty.
- severity: agree, nit. Cosmetic log spam, faithful to Go.
- recommendation-check: valid. Announcing only on the empty -> non-empty transition fixes the spam. Note the leaders set can legitimately change between wraps (a rolloff player raising their score); a transition-only guard would then not re-announce the new membership — acceptable for a nit, but worth knowing.

### F7 Duplicated finish-handling block in both command() arms — CONFIRMED
- evidence: Roll arm (lib.rs:483-498) and Keep arm (lib.rs:500-519) are identical except line 484 `self.player_roll(player)?` vs line 505 `self.keep(player)?`: same `is_finished()` check, same scores-vec construction, same `placings_log` push, same `CommandResponse` build (~15 duplicated lines).
- severity: agree, nit (simplicity).
- recommendation-check: valid. Matching to select the action closure/result then sharing the finish/response tail is a routine refactor; both arms already bind `remaining` identically.

### F8 shoot() drops Go's bounds validation — CONFIRMED
- evidence: `shoot` destructures `let Loc { y, x } = loc` and indexes `self.boards[op][y][x]` directly (battleship-2/src/lib.rs:314-317, writes at :328, :332) with no bounds check. Go validated: `if !IsValidLocation(y, x) { return nil, errors.New("That is not a valid location on the board") }` (brdgme-go/battleship_1/battleship.go:378-380). `Loc` has pub fields and no invariant (lib.rs:157-161). Parser gating verified: the only command path builds `loc` via `Enum::exact(all_locations())` (command.rs:24 for shoot, command.rs:63 for place), and `all_locations()` generates only `y in 0..BOARD_SIZE, x in 0..BOARD_SIZE` (lib.rs:169-177), so every parser-produced Loc is in-bounds — unreachable in the HTTP flow, defense-in-depth only, exactly as the finding states. `place_ship` does bounds-check via `is_valid_location` (lib.rs:277-282), so shoot is also internally inconsistent.
- severity: agree, minor. Not exploitable via command input; a clear parity drop and API-surface footgun.
- recommendation-check: valid. A bounds check at the top of `shoot` returning `GameError::invalid_input` restores Go parity trivially. The private-fields alternative works too but touches `Command::Shoot { loc }` construction and `all_locations()`; the first option is the cheap one.

### F9 Indexing trusts players/Vec lengths; inconsistent with defensive .get() — CONFIRMED
- evidence: panicking indexing: `is_finished` -> `player_hits_remaining` indexes `self.boards[player]` for `p in 0..self.players` (lib.rs:378-379 via :354); `status` indexes `self.left_to_place[p]` (lib.rs:423-424); `placings` indexes via `player_hits_remaining` (lib.rs:387-389); `place_ship` indexes `self.left_to_place[player]` (:270), `self.boards[player]` (:283, :290). Defensive counterparts: `can_place` uses `self.left_to_place.get(player)` (lib.rs:248-251), `player_state` uses `.get(player)...unwrap_or_default()` (lib.rs:460-461), `place_parser` uses `.get(player).cloned().unwrap_or_default()` (command.rs:51). `Game` is all-pub + Deserialize (lib.rs:204-211). Mixed strategy confirmed; not reachable from command input (start() fixes lengths, NUM_PLAYERS enforced at lib.rs:398-405).
- severity: agree, minor. Internal-consistency/maintainability issue plus crafted-state panic surface, same systemic family as F4.
- recommendation-check: valid. "Validate once on load or use .get() consistently" is the right framing; either is safe.

### F10 expect("cell is a ship") in shoot sunk branch — CONFIRMED
- evidence: `let ship = ship_cell.to_ship().expect("cell is a ship");` (lib.rs:331). Currently unreachable: the match at lib.rs:317-330 handles `Cell::Hit | Cell::Miss` and `Cell::Empty`, leaving only the five ship variants, and `to_ship()` (lib.rs:41-50) returns Some for exactly those. A future `Cell` variant would fall into `ship_cell` and panic here.
- severity: agree, nit. Provably unreachable today; hygiene only.
- recommendation-check: valid. Binding via match pattern or handling `None` with an error removes the panic path without behaviour change.

### F11 Ship::all() vs Direction::all() return-type inconsistency — CONFIRMED
- evidence: `pub fn all() -> &'static [Ship]` (lib.rs:64) vs `pub fn all() -> Vec<Direction>` (lib.rs:118-125, allocates a vec of 4 constants).
- severity: agree, nit (consistency).
- recommendation-check: valid with a small caveat: call sites consume owned values — `Enum::partial(Direction::all())` (command.rs:68) and `Ship::all().to_vec()` (lib.rs:411) — so `&'static [Direction]` needs a `.to_vec()` at command.rs:68 (Ship's callers already do this, e.g. `Enum::partial(ships)` from a cloned vec). Mechanical, no bug risk.

### F12 Hit-count helpers return i32 — CONFIRMED
- evidence: `pub fn player_hits_remaining(&self, player: usize) -> i32` (lib.rs:350) and `pub fn player_ship_hits_remaining(&self, player: usize, ship: Ship) -> i32` (lib.rs:362); both count cells, never negative. Consumed by `placings()` metrics `Vec<Vec<i32>>` for `gen_placings` (lib.rs:386-390), matching the stated rationale; also compared `== 0` at lib.rs:333, :379.
- severity: agree, nit.
- recommendation-check: valid. Returning usize and casting at the placings call site works; `== 0` comparisons unaffected. Optional as stated.
