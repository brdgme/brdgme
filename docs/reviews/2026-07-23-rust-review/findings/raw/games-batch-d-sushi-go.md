# Raw findings: game/sushi-go-2 (Worker, read-only review)

Snapshot reviewed: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/sushi-go-2/`.
Variant: **original Sushi Go** (108-card set: tempura/sashimi/dumpling/maki x1-3/nigiri x3/wasabi/pudding/chopsticks), 2-5 players, with the published 2-player dummy variant. Card counts and per-type scoring verified against the official Gamewright rules and are correct. Go port cross-referenced at `/home/beefsack/Development/brdgme/brdgme-go/sushi_go_1/`.

## Findings

### Round 2 passes hands to the right; official Sushi Go passes left every round
- severity: minor
- category: correctness
- location: game/sushi-go-2/src/lib.rs:361 (and log text at lib.rs:293)
- finding: `end_hand` passes hands left in odd rounds and right in even rounds (`self.round % 2 == 1` -> `rotate_left`, else `rotate_right`), and `start_round` announces the direction. The published Sushi Go rules pass hands to the player on the left in *every* round; alternating direction is a 7 Wonders mechanic, not Sushi Go. Judged against official rules this is a rules deviation; NOTE it is inherited verbatim from the Go port (`brdgme-go/sushi_go_1/game.go` EndHand) and is documented in the shipped RULES.md ("Round 1 passes left, round 2 passes right, round 3 passes left"), so it is self-consistent and communicated to players. In a 2-player game direction is irrelevant (hands are simply swapped).
- recommendation: If rules fidelity to the published game is desired, pass left every round (drop the `round % 2` branch). Otherwise close as a deliberate, documented deviation.

### Pudding scoring: all-tied case awards nothing; official rules split the +6 (and, in 2p, the dummy participates in the comparison)
- severity: minor
- category: correctness
- location: game/sushi-go-2/src/lib.rs:492
- finding: In round-3 pudding scoring, if `first == last` (every slot has the same pudding count) no points are awarded. Judged against official rules: in a 3-5 player game, all-tied means everyone splits +6 and everyone splits -6, which nets to 0, so the outcome is equivalent there. But in a 2-player game (no fewest-pudding penalty), an all-tied table should still split the +6 "most puddings" award; this code awards 0. Additionally, in 2-player games the dummy slot is included in the pudding comparison (`pudding` vec is sized `all_players`, lib.rs:460) and can win or dilute the +6; whether the dummy counts for pudding in the official 2-player variant is ambiguous in the published rules (Go port behaves the same way). Edge case, low impact.
- recommendation: In the `first == last` branch, for `players == 2` award `6 / first_players.len()` to the tied slots (or document the deviation). Decide and document whether the dummy participates in pudding scoring in 2p.

### Final placings break score ties by pudding count; official rules leave ties standing
- severity: minor
- category: correctness
- location: game/sushi-go-2/src/lib.rs:273-278
- finding: `placings()` uses `[points, pudding_cards]` as the metric, so a points tie is broken by pudding count. The published Sushi Go rules specify no tiebreaker (tied players share the placement). Judged against official rules this is a deviation; it is inherited from the Go port (`game.go` Placings, "ties broken by number of pudding cards") and is locked in by `test_placings_pudding_tiebreaker` (lib.rs:1461). As a ranked web platform a deterministic tiebreak is defensible, but it is undocumented in RULES.md.
- recommendation: Either document the pudding tiebreak in RULES.md, or drop the pudding component and let `gen_placings` share placings on tied points.

### `test_hand_passing_left` is a vacuous, self-contradicting test
- severity: minor
- category: quality
- location: game/sushi-go-2/src/lib.rs:1398-1416
- finding: The test deliberately triggers `g.command(MICK, "play 1", &n).unwrap_err()`, calls `g.end_hand()` directly, contains trailing comments admitting the scenario doesn't work ("So we can't easily check passing here. Instead test passing with 2-card hands."), and asserts nothing. It provides no coverage and misleads readers into thinking hand-passing is tested there (the real coverage is `test_passing_direction` at lib.rs:1418).
- recommendation: Delete the test, or replace its body with real assertions on hand passing.

### `draw_count` has an unreachable (2, 9) entry and a silent `.unwrap_or(9)` fallback
- severity: minor
- category: quality
- location: game/sushi-go-2/src/lib.rs:140-150
- finding: `player_draw_counts()` includes `(2, 9)`, but `draw_count` is only ever called with `self.all_players` (lib.rs:292), which is 3 in 2-player games — the 2-player entry is dead. The `.unwrap_or(9)` fallback would silently mask any future out-of-range caller instead of failing loudly. (Hand sizes themselves are correct: 3p=9, 4p=8, 5p=7 match the official rules, and the 2p dummy variant correctly deals as a 3-player game.)
- recommendation: Replace the lookup with an exhaustive `match players { 2 | 3 => 9, 4 => 8, 5 => 7, _ => unreachable-free fallback }` (e.g. `debug_assert!` + clamp), or drop the dead entry and document why.

### Pudding hint text claims "least -6" which is false in 2-player games
- severity: nit
- category: correctness
- location: game/sushi-go-2/src/lib.rs:123
- finding: `Card::Pudding.explanation()` returns "end: most 6, least -6" and is shown in every player's hand table (render.rs `hand_table`), but the fewest-pudding penalty is (correctly) not applied in 2-player games (lib.rs:513). 2p players see a rule hint that doesn't apply to their game.
- recommendation: Make the explanation context-aware, or shorten to "end: most 6" / note "(no penalty in 2p)".

### Maki second-place cap `second_players.len() <= 3` is an arbitrary, silent guard
- severity: nit
- category: correctness
- location: game/sushi-go-2/src/lib.rs:440
- finding: Second-place maki points are only awarded if at most 3 players tie for second. The only case this excludes is a 4-way second-place tie in a 5-player game, where official rules would split 3 points rounding down — i.e. 0 points each — so the score outcome is identical; the only effect is that the explanatory log line is silently omitted in that one scenario. The cap looks like a rule but isn't one.
- recommendation: Drop the `<= 3` condition (integer division already yields 0), or add a comment explaining it's a deliberate log-suppression for the 5p 4-way tie.

### `render_name` logic duplicated between lib.rs and render.rs
- severity: nit
- category: consistency
- location: game/sushi-go-2/src/lib.rs:251-260 and game/sushi-go-2/src/render.rs:39-48
- finding: `Game::render_name` (used for log rendering) and the free `render_name` in render.rs (used for board rendering) are byte-for-byte the same "<dummy>"-vs-`N::Player` logic. Two copies to keep in sync. Also both compute `player > players - 1`, which would underflow (panic in debug) if ever called with `players == 0` — not reachable in practice (games are built via `start` with >= 2 players), but the `usize` subtraction is fragile.
- recommendation: Have render.rs call a shared helper (e.g. a free `pub(crate) fn render_name(player, players)` in lib.rs or render.rs) and write the check as `player >= players`.

### Two-card "save one for the dummy" guard evaluates `playing[DUMMY]` before the `players == 2` check
- severity: nit
- category: quality
- location: game/sushi-go-2/src/lib.rs:675-683
- finding: The guard chain is `player == self.controller && self.playing[DUMMY].is_none() && self.players == 2 && ...`. In 3-5 player games `playing[DUMMY]` is player 2's slot, so the condition reads an unrelated player's state before short-circuiting on `players == 2`. It's memory-safe (`playing` always has >= 3 entries) and logically harmless (the `players == 2` conjunct kills it), but it's confusing and would panic if `all_players` ever dropped below 3.
- recommendation: Reorder so `self.players == 2` comes first, matching the guard style in `can_dummy` (lib.rs:248).

### `command()` duplicates the finished-game placings-log block across both match arms
- severity: nit
- category: simplicity
- location: game/sushi-go-2/src/lib.rs:838-874
- finding: The `Command::Play` and `Command::Dummy` arms each repeat the same 12-line sequence: call handler, if finished build `scores` vec, push `placings_log`, build identical `CommandResponse`. Only the handler call differs.
- recommendation: Bind the `Result<Vec<Log>, GameError>` from either arm in one `let mut logs = match ... { Play(..) => self.play(...)?, Dummy(..) => self.dummy(...)? }`, then run the finished-check/response construction once.

## Context notes (not findings)

- Suggest-engine UX: `play_parser` uses `Many::bounded_spaced(Int::bounded(1, max), 1, 2)` (command.rs:42), so the known lib/game defect where the suggest `Many` arm ignores `max` is user-visible here as suggestions possibly offering a third card index. The parse/validation path (`play`, lib.rs:664) enforces the real bound, so this is cosmetic. The lib/game defect itself is tracked elsewhere and not re-reported.
- Parse of `play 1 2 3`: `Many` with max 2 consumes two numbers and leaves ` 3` in `remaining_input`; the command still executes with the first two cards. This is the shared parser's standard remaining-input behavior, tracked cross-unit.

## Cross-references (not findings)

- Alternating pass direction, pudding-tiebreak placings, and dummy participation in maki/pudding scoring are all inherited verbatim from `brdgme-go/sushi_go_1` (verified in `game.go` EndHand/Placings/Score). They are reported above only where they deviate from the *published* rules, per instructions.
- `Game.rng` serde `#[serde(default = "GameRng::from_entropy")]` migration shim (lib.rs:186-189) is documented in-code as deliberate.
- `Card::Played` as a hand-slot placeholder (rather than removing played cards from the hand) mirrors the Go port's `CardPlayed` sentinel; documented in DATA_DOCS.md.
- The 4 binaries under `src/bin/` match the systemic boilerplate pattern exactly (cli/repl/fuzz/http); the http binary's startup `.expect("Invalid socket address")` is the standard pattern. No per-binary issues reported per instructions.

## Verified clean

- No `.unwrap()`/`.expect()`/`panic!()`/`unreachable!()` in runtime paths reachable from player commands (only `unwrap_or*` combinators); unwraps are confined to tests and binary startup.
- Panic-path audit: all player-input indexing (`play`, `dummy`, `play_cards`) is bounds-checked before indexing; `can_play`/`can_dummy` guard every `playing[player]` access; deck-deal `drain(0..dc)` is provably in range for all legal player counts (max 105 of 108 cards dealt).
- Scoring verified correct against official rules: maki first/second with tie splitting (traced multiple orderings), tempura/sashimi set scoring, dumpling triangular capped at 15, nigiri+wasabi (wasabi only triples the *next* nigiri; nigiri played before wasabi is not tripled), pudding most/fewest with 2p penalty exemption, chopsticks double-play + return-to-hand flow (including the rule that chopsticks must have been played in an earlier hand).
- Hand-size synchronization traced for both normal and chopsticks double-play sequences in 2p (with dummy) and 3-5p: returned chopsticks keeps hand sizes in lockstep, so the `hands[0].is_empty()` round-end check cannot strand a player with an unplayable empty hand.
- Hidden-information handling: hands and pending `playing` choices are per-player; the dummy draw is a private log; `dummy_playing` only exposed to the controller.
- Dependencies: lean and standard (workspace path deps + rand/serde/tokio), consistent with other game crates.
