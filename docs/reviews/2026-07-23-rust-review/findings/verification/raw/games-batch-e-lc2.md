# Verification: games-batch-e, lost-cities-2 section (F17-F28)

Verifier: independent read of snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust` (f8763a5). All line
numbers below re-checked against the actual files, not trusted from the
original findings.

## F17 - Status::Finished stats hardcoded to players 0/1

- Verdict: CONFIRMED
- Severity: major - correct (clear defect: wrong API output for a supported
  player count).
- Evidence:
  - `game/lost-cities-2/src/lib.rs:534`:
    `stats: vec![self.player_stats(0), self.player_stats(1)],` inside
    `status()` (lib.rs:530-542).
  - 3 players genuinely supported: `const MIN_PLAYERS: usize = 2; const
    MAX_PLAYERS: usize = 3;` (lib.rs:26-27); `start()` validates
    `(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)` (lib.rs:505);
    `player_counts()` returns `(MIN_PLAYERS..=MAX_PLAYERS).collect()`
    (lib.rs:644-646).
  - Surrounding code is generalized as claimed: `placings()` iterates
    `0..self.players` (lib.rs:491-497), as do `points()` (lib.rs:638-642) and
    the placings-log block in `command()` (lib.rs:619).
  - `player_stats()` guards `player >= self.stats.len()` (lib.rs:457-459), so
    no panic - the 3-player finished status just silently ships a 2-entry
    stats vec against a 3-entry placings vec.

## F18 - player_state() unchecked index, request-reachable panic

- Verdict: CONFIRMED
- Severity: major - correct. Not critical: the `player` value comes from the
  server-side request envelope, not from player command text, so it is
  operator/server-crafted input rather than end-user reachable.
- Evidence:
  - `game/lost-cities-2/src/lib.rs:570`: `hand: self.hands[player].clone(),`
    in `player_state()` (lib.rs:566-572). No bounds check.
  - Request path confirmed: `lib/cmd/src/api.rs:29-32` defines
    `Request::PlayerRender { player: usize, game: String }`;
    `lib/cmd/src/requester/gamer.rs:44-46` dispatches it to
    `handle_player_render::<G>(player, &game)`, and gamer.rs:174 calls
    `game.player_state(player)` with the raw deserialized index.
  - Contrast claim holds: other accessors use guarded access, e.g.
    `self.hands.get_mut(player).ok_or_else(|| GameError::internal(...))`
    (lib.rs:269-271, 298-301, 331-334) and `player_stats`'s explicit bounds
    check (lib.rs:457).
  - Same code in lost-cities-1 at its lib.rs:566 (checked:
    `hand: self.hands[player].clone(),`).

## F19 - Draw logs dropped when draw empties deck

- Verdict: CONFIRMED
- Severity: minor - correct (log-only defect, state unaffected).
- Evidence:
  - `game/lost-cities-2/src/lib.rs:404-446` (`draw_hand_full`): public log
    pushed at lib.rs:434, private "You drew" log at lib.rs:437, both into the
    local `logs`; then:

        441  if self.deck.is_empty() {
        442      self.end_round()
        443  } else {
        444      Ok(logs)
        445  }

    The deck-empty branch returns `end_round()`'s logs only; the local `logs`
    binding is dropped. The final draw of each round (and the round-ending
    "N remaining" count) is never emitted.
  - Accidental-looking, as claimed: `end_round()` itself carefully merges
    `start_round()` logs (lib.rs:188-191), so log preservation was intended
    elsewhere in the same flow.

## F20 - PlayerState.hand documented sorted but unsorted

- Verdict: CONFIRMED
- Severity: minor - correct (doc/API contract mismatch).
- Evidence:
  - Doc comment `game/lost-cities-2/src/lib.rs:102`: "Cards currently in this
    player's hand, sorted by expedition then value."
  - `DATA_DOCS.md:19`: "Cards are sorted by expedition then value."
  - `player_state()` (lib.rs:566-572) clones `self.hands[player]` untouched.
    Only the per-draw batch is sorted - `drawn.sort();` at lib.rs:418 sorts
    the drawn cards for the log, then they were already pushed to the hand in
    deck order (lib.rs:414-417). Cards taken from discards are appended
    unsorted (lib.rs:269-272). `render_hand` sorts a copy for display only
    (render.rs:304-305).

## F21 - Stats.investments never incremented / expeditions write-only

- Verdict: CONFIRMED
- Severity: minor - correct.
- Evidence:
  - `grep -rn investments src/` yields exactly one hit:
    `lib.rs:51: pub investments: usize,` - declared, serialized (Stats
    derives Serialize/Deserialize, lib.rs:44), never written.
  - `stats[player].expeditions += 1` at lib.rs:383 (inside `play()`, gated on
    `player_expedition.is_empty()`), but `player_stats()` (lib.rs:455-489)
    surfaces only Plays, Discards, Draws, Takes - `expeditions` is
    write-only.

## F22 - unreachable!() on player counts outside 2..=3

- Verdict: CONFIRMED
- Severity: minor - correct (panic requires a malformed server-supplied state
  blob or misuse of the pub `score` fn, not player input).
- Evidence:
  - `_ => unreachable!()` in `expedition_cost` (lib.rs:677), `hand_size`
    (lib.rs:685), `expedition_bonus_size` (lib.rs:693) - i.e. the claimed
    lib.rs:673-695 span - and in `render_tableau`'s `match self.players`
    (render.rs:187).
  - `Game` derives `Deserialize` with no validation (lib.rs:55-56); the only
    player-count validation is in `Gamer::start` (lib.rs:505-511).
  - State-blob claim confirmed: `lib/cmd/src/api.rs:20-32` - `Request::Play`,
    `Status`, `PubRender`, `PlayerRender` all carry `game: String`, and
    `requester/gamer.rs:27-47` does `serde_json::from_str(game)` straight
    into `G` with no invariant check. A blob with `players: 4` panics on the
    next draw (`hand_size`), score (`expedition_cost`), or render
    (render.rs:187).
  - `pub fn score(players, ...)` (lib.rs:697-699) calls `expedition_cost` /
    `expedition_bonus_size` directly, so any out-of-range count panics there
    too.

## F23 - Perspective index % MAX_PLAYERS instead of % self.players

- Verdict: CONFIRMED
- Severity: nit - correct (latent; no reachable mis-render today).
- Evidence:
  - `game/lost-cities-2/src/render.rs:130`:
    `let p = player.unwrap_or(0) % MAX_PLAYERS;` in
    `PubState::render_tableau`. In a 2-player game an index of 2 survives
    (`2 % 3 == 2`) and `self.expeditions.get(2)` renders nothing for the
    bottom half.
  - Latency claim holds: the only `Some(p)` caller is
    `PlayerState::render` (render.rs:122-126), whose state comes from
    `player_state(player)`, which panics first on out-of-range players
    (lib.rs:570).
  - Internal inconsistency supporting the finding: the scores section in the
    same file clamps properly - `Some(p) if p < pub_state.players => p, _ =>
    0` (render.rs:51-54) - so `% MAX_PLAYERS` is the odd one out. (The
    lost-cities-1 comparison in the original finding matches this clamp
    pattern in spirit; -1 has no MAX_PLAYERS modulo.)

## F24 - Game-over log regressed vs lost-cities-1

- Verdict: CONFIRMED
- Severity: nit - correct (cosmetic/log quality).
- Evidence:
  - lost-cities-2 `game_over_log` (lib.rs:198-200):
    `Log::public(vec![N::Bold(vec![N::text("The game is over.")])])`.
  - lost-cities-1 `game_over_log` (its lib.rs:171-191): builds
    "The game is over, " + winner + `" won by {} points"` (using
    `self.winners()` at -1 lib.rs:173/491 and `opponent()` at -1
    lib.rs:500), or "scores tied at {}".
  - The margin computation in -1 is inherently 2-player (`opponent(p)`), so
    the generalization to 3 players plausibly motivated the cut - but the
    winner announcement itself is generalizable and -2's `leaders()`
    (lib.rs:120-134) already computes the top scorer(s), as claimed.

## F25 - Discard piles expose only top card

- Verdict: CONFIRMED (evidence basis: external - official Lost Cities rules;
  code side verified rigorously)
- Severity: nit - correct, given it is plausibly a deliberate simplification;
  the recommendation to decide-and-document is proportionate.
- Evidence (code side):
  - Full piles are in state: `Game.discards: Vec<Card>` (lib.rs:61).
  - `PubState.discards: HashMap<Expedition, Value>` (lib.rs:87) - top value
    only - populated in `pub_state()` via `card::last_expedition(&self.discards, e)`
    (lib.rs:551-559).
  - Renderer shows only the top card per pile (render.rs:213-220).
  - Neither RULES.md nor DATA_DOCS.md documents the reduced visibility:
    RULES.md:49 just says "Take the top card from any shared discard pile";
    DATA_DOCS.md describes `discards` as "The top (most recently discarded)
    card value on each expedition's shared discard pile" - accurate about the
    API, silent on the deviation from the physical game's face-up piles.
  - Same shape in lost-cities-1 (its PubState also maps expedition to a
    single value).

## F26 - usize underflow in draw-count computation

- Verdict: CONFIRMED
- Severity: nit - correct (unreachable through normal play; hand size is
  restored to exactly `hand_size` each turn cycle).
- Evidence:
  - `game/lost-cities-2/src/lib.rs:408`:
    `let mut num = hand_size(self.players) - hand.len();` - underflow panics
    in debug, wraps in release (then clamped to deck length at
    lib.rs:409-412, so release behaviour would be a full-deck drain into one
    hand).

## F27 - k8s GameVersion blurb still says two-player

- Verdict: CONFIRMED (one detail off: the blurb is at yaml line 9, not line 1
  as the location field implies; immaterial)
- Severity: minor - correct (user-facing incorrect product info).
- Evidence:
  - `k8s/base/game/lost-cities-2/game-version.yaml:9`:
    `blurb: "... A tense two-player card game of investment and restraint."`
  - Identical blurb in `k8s/base/game/lost-cities-1/game-version.yaml:9`
    (whose manifest carries `isDeprecated: true` at line 10), confirming the
    copied-verbatim claim.
  - Crate advertises 2-3: `player_counts()` = `(2..=3).collect()`
    (lib.rs:644-646, via MIN_PLAYERS/MAX_PLAYERS at lib.rs:26-27); RULES.md
    documents 7-card hands / 15-cost / 3-player column (RULES.md:23 and the
    scoring table).

## F28 - Stale build-release / .rls.toml files

- Verdict: CONFIRMED, with one detail ADJUSTED
- Severity: nit - correct.
- Evidence:
  - `game/lost-cities-2/build-release` exists: a `clux/muslrust` docker build
    + lambda packaging script (`cp target/x86_64-unknown-linux-musl/release/cli
    lambda/game`, curls `brdgme/lambda/master/index.js`) - obsolete under the
    current `rust/Dockerfile` (present at repo `rust/Dockerfile`).
  - `.rls.toml` exists (16 bytes) and configures the retired RLS.
  - Adjustment: the file is NOT malformed. `od -c` shows exactly
    `build_lib = true` with no trailing newline; the original finding's
    `build_lib = truetarget` is a cat-concatenation artifact (the next byte
    printed came from `.gitignore`'s `target` line). Missing trailing
    newline, valid TOML.
  - `.gitignore` still carries the matching cruft (`lambda` entry), also
    stale.

## Summary

| Finding | Verdict | Severity |
|---|---|---|
| F17 stats hardcode 0/1 | CONFIRMED | major (keep) |
| F18 player_state panic | CONFIRMED | major (keep) |
| F19 draw logs dropped | CONFIRMED | minor (keep) |
| F20 hand doc sorted | CONFIRMED | minor (keep) |
| F21 dead stats fields | CONFIRMED | minor (keep) |
| F22 unreachable!() 2..=3 | CONFIRMED | minor (keep) |
| F23 % MAX_PLAYERS | CONFIRMED | nit (keep) |
| F24 game-over log regression | CONFIRMED | nit (keep) |
| F25 discard top-card only | CONFIRMED (external basis) | nit (keep) |
| F26 draw-count underflow | CONFIRMED | nit (keep) |
| F27 k8s blurb two-player | CONFIRMED (line 9 not 1) | minor (keep) |
| F28 stale build files | CONFIRMED / detail ADJUSTED (.rls.toml not malformed) | nit (keep) |
