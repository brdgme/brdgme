# Verification: games-batch-e (age-of-war-2 F10-F16, red7-1 F29-F35)

Verifier: independent worker, snapshot commit f8763a5.
All paths relative to /home/beefsack/Development/brdgme-review-snapshot/rust
unless noted. Go original: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go/age_of_war_1.

## F10 - age-of-war-2: panicking unwrap/expect cluster in runtime paths

Verdict: CONFIRMED (with one scope note)

Evidence - all six sites exist as claimed:
1. `game/age-of-war-2/src/lib.rs:132` (`scores()`):
   `let clan_idx = ALL_CLANS.iter().position(|&cl| cl == c.clan).unwrap();`
   Every castle's clan is a member of `ALL_CLANS`, so this is provably total.
2. `lib.rs:218-220` (`check_end_of_turn`):
   `prior_owner.expect("conquered castle has an owner")` - only reached when
   `was_conquered` (lib.rs:209,216); conquest always sets both `conquered[idx]`
   and `castle_owners[idx]` together (lib.rs:223-224), so the invariant holds.
3. `lib.rs:332-334` (`line_action`):
   `self.currently_attacking.expect("can_line implies currently_attacking")` -
   guarded by `can_line` check at lib.rs:327-331, and `can_line` (lib.rs:81-83)
   requires `currently_attacking.is_some()`.
4. `command.rs:89` (`line_parser`):
   `self.currently_attacking.expect("currently attacking")` - only called when
   `can_line(player)` (command.rs:39-41).
5. `render.rs:48`:
   `state.castle_owners[idx].expect("conquered castle has an owner")` inside
   `if state.conquered[idx]`.
6. `render.rs:110`:
   `N::Player(by.expect("conquered clan has an owner"))` inside
   `if conquered` from `clan_conquered` (which returns `Some` player when true
   and all owners match).

Each invariant genuinely holds for all command-flow-maintained state, as the
finding says. These run under the Play/Render request handlers
(`lib/cmd/src/requester/gamer.rs:131,161,174`), and there is no
`catch_unwind` anywhere in `lib/` (grep confirms), so a panic kills the
request.

Scope note: CODING.md's rule (docs/CODING.md:46-49) literally names "server
request handlers, database functions, and Leptos component code". Game-crate
code is in scope only via being executed by the game-service request handler;
the finding's framing ("makes no invariant exception") is fair but the rule's
letter does not name game crates. Does not change the verdict.

Severity: minor is right (all sites latent, none reachable from crafted
input; consistency/robustness concern).

## F11 - age-of-war-2: `completed_lines: HashSet<usize>` serializes nondeterministically

Verdict: CONFIRMED

Evidence:
- `Game` derives `Serialize, Deserialize` (lib.rs:24) and IS the persisted
  blob: `GameResponse::from_gamer` does `state: serde_json::to_string(gamer)?`
  (`lib/cmd/src/api.rs:183-188`).
- `pub completed_lines: HashSet<usize>` at lib.rs:35 (finding cites :35 in
  location line as lib.rs:35 - the location header says :35; body matches).
  Std `HashSet` uses `RandomState`, randomized per-instance, so iteration
  (and thus serde serialization) order is nondeterministic.
- Pub-view workaround exactly as claimed, lib.rs:434-435:
  `let mut completed: Vec<usize> = self.completed_lines.iter().cloned().collect();`
  `completed.sort_unstable();`

Severity: minor is right (no gameplay defect; breaks byte-level state
diffing/hashing only).

## F12 - age-of-war-2: "not your turn" as unstructured invalid_input

Verdict: CONFIRMED

Evidence:
- lib.rs:461-464: `None => return Err(GameError::invalid_input("not your turn"))`.
- `command_parser` returns `None` exactly when none of can_attack/can_line/
  can_roll hold (command.rs:45-49); `can_roll` is `current_player == player`
  (lib.rs:86-88), so None <=> not the player's turn.
- `GameError::NotYourTurn` exists: `lib/game/src/errors.rs:16`.

Severity: nit is right.

## F13 - age-of-war-2: placings-log tail triplicated

Verdict: CONFIRMED

Evidence: identical block (build `scores` vec from `self.scores()`, push
`placings_log(&self.calc_placings(), Some(&scores))` when `self.is_finished()`)
appears three times: lib.rs:473-482 (Attack arm), lib.rs:491-500 (Line arm),
lib.rs:509-518 (Roll arm). Only the `self.attack(..)` / `self.line_action(..)`
/ `self.roll_action(..)` call differs.

Severity: nit is defensible (3 copies of ~10 lines; the love-letter analogue
with 8 copies was rated major - consistent ordering).

## F14 - age-of-war-2: finished games keep emitting duplicate placings logs

Verdict: CONFIRMED

Evidence:
- `can_roll` (lib.rs:86-88) is only `self.current_player == player` - no
  finished check. The preserved-quirk test
  `command_after_finished_still_accepted_go_quirk` (lib.rs:816-826) asserts
  `g.command(g.current_player, "roll", &p).is_ok()` on a fully conquered board.
- On a finished game, `roll_action` -> `check_end_of_turn`: with
  `currently_attacking == None`, the else branch (lib.rs:260-277) `continue`s
  every castle (every clan is conquered), so it falls through to
  `failed_attack_message` + `next_turn` (lib.rs:275-276) - `current_player`
  advances.
- Back in `command()`, `is_finished()` is still true, so another
  `placings_log` is pushed (lib.rs:509-518). Every subsequent `roll` repeats
  this.
- Go comparison confirmed: `age_of_war_1/game.go:50-64` - `Command` just
  dispatches to Attack/Line/RollCommand and never appends placings logs, so
  the amplification is new in the Rust port, exactly as claimed.

Severity: nit is right (log spam in an already-finished game; state placings
remain correct).

## F15 - age-of-war-2: `clan_conquered` duplicated between Game and renderer

Verdict: CONFIRMED

Evidence: `Game::clan_conquered` at lib.rs:93-113 and free fn
`clan_conquered(state: &PubState, ...)` at render.rs:10-30 are line-for-line
the same algorithm, both including the quirk of returning `(false, player)`
with a possibly-stale player value (lib.rs:106-108 / render.rs:23-25). The
render copy even carries the doc comment "Port of Game.ClanConquered
(game.go), operating on the public fields" (render.rs:8-9).

Severity: nit is right.

## F16 - age-of-war-2: "discard one dice" help text

Verdict: CONFIRMED

Evidence: command.rs:113-119 -
`Doc::name_desc("roll", "discard one dice and roll the rest", Token::new("roll"))`
(the string is on command.rs:117). Go original has the identical string:
`age_of_war_1/command.go:108-112` (`Desc: "discard one dice and roll the rest"`),
confirming "carried verbatim from Go".

Severity: nit is right.

## F29 - red7-1: CardParser panics on non-ASCII input via byte-index slicing

Verdict: CONFIRMED

Evidence:
- `game/red7-1/src/command.rs:23-24`: the guard counts CHARS:
  `let chars: Vec<char> = input.chars().collect(); if chars.len() < 2 { ... }`.
- command.rs:31,34-35 then slice BYTES:
  `match Card::parse(&input[..2])`, `consumed: &input[..2]`,
  `remaining: &input[2..]`.
- Concrete reproduction of the reasoning: input `"r€"` is 2 chars but 4 bytes
  (0x72, then E2 82 AC); `&input[..2]` puts the cut at byte 2, inside the
  3-byte `€` - Rust str slicing panics ("byte index 2 is not a char
  boundary"). `"€5"` panics identically (cut at byte 2 inside the leading
  3-byte char). Any input whose byte-2 offset is mid-char panics; the
  chars-vs-bytes mismatch is exactly as described.
- `Card::parse` itself (card.rs:113-124) is char-safe (`input.chars()`); the
  panic is purely the slicing in `CardParser::parse` - crate-local, as
  claimed.
- Reachability from the Play endpoint, traced end to end:
  `Request::Play` -> `handle_play` (`lib/cmd/src/requester/gamer.rs:125-131`)
  passes the raw command string into `game.command(player, command, names)`;
  red7's `command()` (lib.rs:451-459) calls
  `self.command_parser(player).parse(input, players)`; the play/discard
  parsers are `Chain2(Token("play"/"discard"), AfterSpace(CardParser))`
  (command.rs:62-87). `Token` matching on ASCII literals and AfterSpace do
  not cut mid-char here, so `CardParser` receives the tail `"r€"` and panics.
  So `play r€` or `discard €5` from the current player's command box reaches
  the panic. No `catch_unwind` exists anywhere in `lib/` (grep), so the
  panic kills the game-service request.
- Gating: `command_parser` requires `!self.finished && current_player == player`
  (command.rs:56-58), so the attacker must be the player to move - i.e. any
  ordinary participant on their turn. Reachable in normal operation.

Severity: critical is justified under the charter (player-input-triggerable
panic that kills the game service request = availability bug from crafted
input; "bug/security" bucket). If the panel wants to reserve critical for
data-loss, major would be the floor - but as the batch's only
remote-input-reachable panic, critical is defensible and I concur.

## F30 - red7-1: player with zero rule-fulfilling cards treated as winning

Verdict: CONFIRMED (rules premise on external basis)

Evidence - concrete trace of the given example (Green rule, p0=[b5], p1=[r7]):
- `leader_with_suit(Green)` (lib.rs:237-252): `rule_fn = most_even_cards`
  (card.rs:290). b5 (rank 5, odd) -> `[]`; r7 (rank 7, odd) -> `[]`. So
  `palettes = [[], []]`.
- `leader(&palettes)` (card.rs:297-317): `leader_idx = 0`,
  `leader_palette = []`. Iteration i=1: `l_max = (0,0)` and `i_max = (0,0)`
  via `.max().unwrap_or((0, 0))` (card.rs:305-310); the condition at
  card.rs:311 `p.len() > leader_palette.len() || (p.len() == leader_palette.len() && i_max > l_max)`
  is `0 > 0 || (0 == 0 && (0,0) > (0,0))` = false. Leader stays index 0 - the
  FIRST non-eliminated player, with an empty winning set. Trace matches the
  finding exactly.
- Only Green (`most_even_cards`, card.rs:230-232) and Violet
  (`most_cards_below_4`, card.rs:278-283) can return empty for a non-empty
  palette; the other five rules always return >=1 card - the finding's
  "possible under Green or Violet" qualifier is accurate.
- Consequence (1): `end_turn` (lib.rs:154-173) eliminates the current player
  only when `leader_idx != self.current_player` (lib.rs:156-163); the
  zero-qualifying first player is the "leader" and survives `done`
  (done -> end_turn, lib.rs:355-369). Confirmed. (Finding cites
  lib.rs:154-164; the check is at 155-164 within end_turn - close enough.)
- Consequence (2): `discard` pre-check at lib.rs:325-330:
  `let (leader_idx, _) = self.leader_with_suit(card.suit); if leader_idx != player { return Err(...) }`.
  When all rule sets under the new suit are empty, `leader_idx` is the
  lowest-index non-eliminated player, so only that player can make the
  discard - confirmed as described.
- Consequence (3): `end_round` (lib.rs:175-216): `pts = points(&leader_palette)`
  = 0 for an empty set; `scored_cards[leader_idx].extend(&leader_palette)`
  extends with nothing; the log announces winning "for 0 points". Confirmed
  (lib.rs:176-179).
- Documentation check: neither RULES.md nor DATA_DOCS.md documents an
  empty-set/"cannot win" rule or a deviation from it - RULES.md:31-32 only
  says "if you are not the leader ... you are eliminated". So the deviation
  is undocumented. The claim that official Red7 says a player with no
  rule-fulfilling card cannot win rests on the external rulebook -
  evidence basis: external. The code-behavior side is fully verified.

Severity: major is right (clear rules-correctness defect changing
elimination/discard legality/scoring in a reachable game state).

## F31 - red7-1: DATA_DOCS.md tie-break description contradicts code

Verdict: CONFIRMED

Evidence:
- DATA_DOCS.md:36 reads exactly: "Ties within a rule are broken by the
  highest card in the winning set, then by the highest card overall in the
  palette."
- Code: `leader()` (card.rs:297-317) is only ever fed the rule-filtered
  winning sets (`palettes.push(rule_fn(&self.palettes[p]))`, lib.rs:247) and
  compares set size then `max(rank_key)` of the winning set (card.rs:305-311).
  There is no fallback comparison against the full palette anywhere - the
  second clause is unimplemented. On fully-tied (including all-empty) sets
  the strict `>` keeps the first player by index.
- The first clause IS implemented (max rank_key = value then color ordinal,
  card.rs:134-136), so only the second clause is fiction, matching the
  finding.
- "does not exist in official rules": external basis, not contradicted by
  anything in-repo.

Severity: minor is right (wrong API documentation consumed by bots).

## F32 - red7-1: RULES.md undersells turn structure and misdescribes scoring

Verdict: CONFIRMED (claim 1 wording slightly generous, substance stands)

Evidence:
- Claim (1): RULES.md:19-32 ("Turn") presents a numbered "On your turn you
  may:" list of Play / Discard / Done and never states that play-then-discard
  in the same turn is legal. The code permits it: `can_discard`
  (lib.rs:287-289) is `self.current_player == player && !self.finished` - it
  ignores `has_played` (unlike `can_play`, lib.rs:283-285, which requires
  `!self.has_played`). So a player can `play b4` then `discard r7` in one
  turn. Minor wording note: the doc's numbered "may" list doesn't literally
  say "alternatives" - it is silent rather than contradictory - but the
  material claim (the sanctioned combo is undocumented while being the
  game's strongest move) stands. Officially-sanctioned status: external
  basis.
- Claim (2): RULES.md:48-49 says the remaining player "scores their palette
  cards". Code scores only the rule-meeting subset: `end_round` uses
  `self.leader()` whose palette is `rule_fn(&self.palettes[p])`
  (lib.rs:176-179 via lib.rs:247), i.e. only cards meeting the current rule
  are moved to `scored_cards` and counted. Confirmed.

Severity: minor is right.

## F33 - red7-1: aliased `PubCard`/`PubSuit` re-export unused and non-conventional

Verdict: CONFIRMED

Evidence:
- lib.rs:16: `pub use card::{Card as PubCard, Suit as PubSuit};`
- Workspace-wide grep over the snapshot `rust/` tree finds `PubCard`/`PubSuit`
  ONLY at that definition line - zero references anywhere else.
- Sibling convention confirmed by grep: alhambra-1, seven-wonders-1, and
  starship-catan-1 all use `pub use card::*;` (each at src/lib.rs:5); red7-1
  is the only crate using the aliased form.

Severity: nit is right.

## F34 - red7-1: `leader_with_suit` indexes `player_map[l_index]` - panics if all eliminated

Verdict: CONFIRMED

Evidence:
- lib.rs:237-252: `player_map`/`palettes` are populated only for
  non-eliminated players (lib.rs:242-248); `card::leader` returns `(0, vec![])`
  for an empty slice (card.rs:298-300); then lib.rs:251
  `(player_map[l_index], palette)` indexes `player_map[0]` - panics when
  `player_map` is empty (all players eliminated).
- Call-site audit confirms unreachability today: `end_turn` calls `leader()`
  only when `!self.eliminated[self.current_player]` (lib.rs:155-156);
  `end_round` fires when `remaining_players().len() == 1` (lib.rs:166-168);
  `start_round` resets `eliminated` to all-false (lib.rs:86) before the
  leader call at lib.rs:100; `discard`'s caller is the current player, who is
  never eliminated while current. Implicit, undocumented precondition -
  exactly as the finding says.

Severity: nit is right.

## F35 - red7-1: `end_points` underflows for player counts above 10

Verdict: CONFIRMED

Evidence: lib.rs:22-24:
`pub fn end_points(players: usize) -> u32 { (50 - players * 5) as u32 }`
`players * 5 > 50` (players >= 11) makes the usize subtraction overflow:
panic in debug builds, wrap (then truncating `as u32` of a huge value) in
release. All in-crate callers pass `self.num_players`, validated to 2..=4 at
`Game::start` (lib.rs:377-383) - `MIN_PLAYERS = 2`, `MAX_PLAYERS = 4`
(lib.rs:19-20) - so unreachable in practice, but it is `pub` with no guard
or documented precondition, as claimed.

Severity: nit is right.

## Summary

| id  | verdict   | severity change |
|-----|-----------|-----------------|
| F10 | CONFIRMED | none (minor) - note: CODING.md's letter names server handlers, game crates in scope only via request path |
| F11 | CONFIRMED | none (minor) |
| F12 | CONFIRMED | none (nit) |
| F13 | CONFIRMED | none (nit) |
| F14 | CONFIRMED | none (nit) |
| F15 | CONFIRMED | none (nit) |
| F16 | CONFIRMED | none (nit) |
| F29 | CONFIRMED | none (critical stands; major would be the floor if critical is reserved for data-loss) |
| F30 | CONFIRMED (external rules basis) | none (major) |
| F31 | CONFIRMED | none (minor) |
| F32 | CONFIRMED | none (minor); claim-1 wording slightly generous, substance intact |
| F33 | CONFIRMED | none (nit) |
| F34 | CONFIRMED | none (nit) |
| F35 | CONFIRMED | none (nit) |
