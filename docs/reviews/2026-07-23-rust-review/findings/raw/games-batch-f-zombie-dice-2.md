# zombie-dice-2 review findings

Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/zombie-dice-2/` (lib.rs 995, command.rs 51, render.rs 151, tests/contract.rs 7 LOC).
Go original: `brdgme-go/zombie_dice_1/` (game.go, dice.go, render.go, command.go) read in full and compared.

Overall: the port is faithful and high quality. Dice faces/counts (6G/4Y/3R, G=3B2F1S, Y=2B2F2S, R=1B2F3S), bust handling, 13-brain wrap-to-player-0 win check, and the tiebreak rolloff all match the Go original and (except where noted) the official SJG rules. No unwrap/expect/unreachable outside tests; the one unwrap in render.rs:15 is guarded by an explicit `n == 1` check. Command input from players cannot reach a panic — all parse failures become `GameError::invalid_input`. Tests are thorough, including seed-searched deterministic rolloff tests.

### Cup draw order leaked to all players in PubState
- severity: major
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:193-194 (`pub cup: Vec<Dice>` in `PubState`, doc says "in draw order"), populated at lib.rs:443; documented in DATA_DOCS.md:8
- finding: `pub_state()` exposes the full shuffled cup **in draw order** to every player. In the physical game (and the official rules) dice are drawn from the cup "without looking" — the composition of remaining dice is deducible public info, but the *order* in which colours come out is hidden. Because the cup is only shuffled at turn start / refill, any client can read the exact colours of the next dice to be drawn (e.g. "the next two draws are red") and make roll/keep decisions with perfect foreknowledge. This materially changes the game. The Go original returned `nil` from `PubState()`, so nothing leaked; this is a Rust-side divergence. DATA_DOCS.md:18 even claims "Zombie Dice has no hidden information per player", which is inaccurate — the cup draw order is hidden information.
- recommendation: don't serialize cup order in `PubState`. Either expose only per-colour counts (matching `render_cup`, which already renders only counts at render.rs:51-73), or sort/canonicalize the cup vector in `pub_state()` so order carries no information, and fix the DATA_DOCS.md "no hidden information" claim.

### Cup refill returns shotgun dice to the cup (deviates from official rules; faithful Go port)
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:242-250 (`take_dice` refill branch); documented behaviour in RULES.md:31
- finding: When the cup has fewer dice than needed, ALL kept dice (brains **and** shotguns) are returned to the cup (`let returned: Vec<Dice> = self.kept.iter().map(|dr| dr.dice).collect();`). The official SJG rulebook says: "If you don't have three dice left in the cup, make a note of how many Brains you have and put them all in the cup (**keep the Shotguns in front of you**). Then continue." — only brain dice go back; shotgun dice stay out for the rest of the turn. Returning shotguns lets the same physical die shotgun the player again within one turn and slightly alters the dice-odds on long turns. This is a faithful port of Go `TakeDice` (game.go:105-114), which has the same deviation, and RULES.md documents the ported behaviour — so this is a cross-reference to a preserved Go quirk, not a fresh porting bug.
- recommendation: decide deliberately whether to keep the Go behaviour (then nothing to do; maybe note the official-rules deviation in RULES.md) or match the official rule by returning only `Face::Brain` kept dice to the cup.

### Tiebreak rolloff state not exposed in PubState / render
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:185-207 (`PubState` fields), lib.rs:438-455 (`pub_state()`), render.rs:92-145
- finding: `Game::roll_off_players` (lib.rs:173) is never copied into `PubState`, and `render()` shows nothing about an active rolloff. The only place a client learns a tiebreaker round started is the transient "tie breaker round!" log (lib.rs:279-285). A client that joins/renders mid-rolloff cannot tell that a rolloff is active or which players are participating (non-participants' turns are silently skipped). Go had no structured pub state at all (`PubState()` returned nil), so there is nothing to be faithful to here — the Rust PubState is new and simply incomplete.
- recommendation: add `roll_off_players: Vec<usize>` (or an `Option`) to `PubState`, populate it in `pub_state()`, and consider surfacing it in `render()` and DATA_DOCS.md.

### `take_dice` can panic via `drain(..n)` if the total-dice invariant is broken (crafted state)
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:251 (`let taken: Vec<Dice> = self.cup.drain(..n).collect();`)
- finding: `drain(..n)` panics when `self.cup.len() < n` after the refill branch. In legitimate play this is unreachable: per turn the 13 dice are partitioned across cup/kept/current_roll, `current_roll` holds at most 3 footprints, so after refilling from `kept`, `cup.len() >= n` always holds. However, `Game` derives `Deserialize` with all-pub fields and no validation; a deserialized state with `cup` and `kept` both short/empty (or an over-long `current_roll`) would panic the HTTP request on the next `roll`. Go panics identically (`g.Cup[:n]` slice, game.go:116), so this is a preserved-Go panic path, cross-reference only.
- recommendation: if defence-in-depth for deserialized state is desired, clamp `n` to `cup.len()` after refill or return a `GameError` instead of panicking; otherwise acceptable given the invariant.

### Unchecked `scores` indexing and `% players` assume deserialized-state invariants
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:368 (`self.scores[self.current_turn] += ...`), lib.rs:384-390 (`leaders()` indexes `self.scores[p]` for `p in 0..self.players`), lib.rs:267 (`(self.current_turn + 1) % self.players` panics on `players == 0`), render.rs:132-139 (render indexes `self.scores[p]` for `p in 0..self.players`)
- finding: Several index/arithmetic paths trust that `scores.len() == players`, `current_turn < players`, and `players > 0`. `start()` establishes these (lib.rs:406-422), and player-supplied *commands* cannot break them, but `Game`/`PubState` derive `Deserialize` with no validation, so a malformed stored/POSTed state (e.g. `players: 4, scores: [0]`, or `players: 0`) panics the request — a panic kills the HTTP request in the per-game service. No panics are reachable from crafted *command input* alone; this is purely a state-deserialization robustness gap, common to most ported crates.
- recommendation: either add a `validate()` after deserialize (length/turn sanity) in the service layer, or use `get()`-style access with `GameError` on the hot paths. Low priority if game state is only ever server-produced.

### Unbounded recursion chain on repeated busts (theoretical)
- severity: nit
- category: quality
- location: game/zombie-dice-2/src/lib.rs:347 (`roll()` bust calls `next_player()`), lib.rs:288-291 (`next_player()` recursion for rolloff skip), lib.rs:262 (`start_turn()` calls `roll()`)
- finding: `roll -> next_player -> start_turn -> roll` recurses once per consecutive busted turn; the rolloff-skip recursion is bounded by player count (<=8). An arbitrarily long run of consecutive 3-shotgun busts would grow the stack without bound — probability zero in practice (each roll's bust is independent, ~few % chance), and RNG is server-side so not adversarially steerable. Same recursion shape as Go.
- recommendation: none required; could be converted to a loop if ever touched for other reasons.

### Rolloff tie announcement re-logged on every wrap while still tied
- severity: nit
- category: quality
- location: game/zombie-dice-2/src/lib.rs:276-286
- finding: Each time play wraps to player 0 with the score still tied at >=13, the "It's a tied score ... tie breaker round!" log is emitted again and `roll_off_players` is reassigned, so a long rolloff spams the log with repeated identical announcements. Faithful to Go (game.go:141-153); cosmetic only.
- recommendation: optionally only announce when `roll_off_players` transitions from empty to non-empty.

### Duplicated finish-handling block in both `command()` arms
- severity: nit
- category: simplicity
- location: game/zombie-dice-2/src/lib.rs:483-499 (Roll arm) and lib.rs:500-520 (Keep arm)
- finding: The two match arms are byte-for-byte identical except for `player_roll` vs `keep`: both build the same scores vec and push `placings_log` when finished. ~15 duplicated lines that must stay in sync.
- recommendation: match only to select the action closure (or map `Command` to a method), then run the shared finish/response code once.

### Cargo.toml: binary-only deps declared as library deps (known systemic issue)
- severity: nit
- category: dependencies
- location: game/zombie-dice-2/Cargo.toml:9-10,16 (`brdgme_cmd`, `brdgme_fuzz`, `tokio` in `[dependencies]`)
- finding: `brdgme_cmd`, `brdgme_fuzz`, and `tokio` are only used by the `src/bin/` targets, not the library. Cross-reference to the known systemic "binary-only deps declared as library deps" issue tracked elsewhere — noted here only because this crate is a consumer.
- recommendation: tracked systemically; no per-crate action.

## Cross-references verified (no finding)
- Go `Leaders()` initializes `players = []int{0}` causing duplicate player 0 in the all-zero-scores case (game.go:242); the Rust port (lib.rs:381-394) correctly starts empty. Rust-side improvement, and the Go bug is unreachable anyway (leaders only matter at score >= 13).
- Rust `can_roll`/`can_keep` add `!self.finished` (lib.rs:216-222) where Go's `CanRoll`/`CanKeep` do not; behaviour is equivalent because `command_parser` returns `None` when finished (command.rs:13-15). Benign divergence / slight hardening.
- Face display names ("Brain"/"Shot"/"Run", lib.rs:92-100) match Go `DiceFaceStrings` exactly.
- `roll_off_players: Vec<usize>` faithfully ports Go's `map[int]bool` nil-vs-set semantics via empty-vs-non-empty (lib.rs:169-173, 227-229), including re-checking leaders at every wrap to player 0 even when player 0 is skipped in the rolloff.
- Command set (`roll`/`keep`, token parsers, "not expecting any commands at the moment" error) matches Go command.go; parse errors become `GameError`, no panic path from crafted command strings.
- Serde migration shim `#[serde(default = "GameRng::from_entropy")]` (lib.rs:179-182) is commented as temporary — fine.

## Clean areas
- Dice face tables, cup composition (13 = 6G/4Y/3R), 3-dice rolls, footprint re-roll, 3-shotgun bust losing all round brains, keep-banking, 13+ win check only at wrap to player 0 (finish-the-round semantics), and placings via shared `gen_placings` all verified correct against the official rules PDF and the Go source.
- No `unwrap`/`expect`/`panic!`/`unreachable!`/indexing reachable from player command input; parser failures are all converted to `GameError::invalid_input`.
- In-crate tests (lib.rs:558-994) are unusually thorough: face/cup counts, refill, bust cascade, rolloff start/skip/resolve via seed search, placings ties, command happy/error paths, pub_state field capture. `tests/contract.rs` runs the shared gamer contract.
