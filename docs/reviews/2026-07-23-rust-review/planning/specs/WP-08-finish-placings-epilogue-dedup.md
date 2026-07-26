# WP-08: finish/placings epilogue dedup sweep

**Findings:** a F6 (minor), e F1 (major), b F11, b F22, b F33, c F6, d F21,
d F33, e F13, e F14, f F7, f F35 (nits). All CONFIRMED by verification; none
are already fixed in live code.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** These crates are
> under concurrent edit; no line numbers are cited on purpose.

## 0. Refactor shape - DECIDED: per-crate private helper, no new lib/game API

Live code was sampled across nine of these crates. Only the last line -
`logs.push(placings_log(&placings, Some(&scores)))` - is genuinely common, and
`brdgme_game::placings_log` is already that shared helper. Everything above it
diverges: the finished predicate (`self.is_finished()` vs rtta's `self.finished`
field), the scores expression (`player_points`, `player_total_money`,
`player_vp`, `scores()`, token sums, `player_score`) and the placings expression
(`self.placings()`, `self.calc_placings()`, or a local `gen_placings(&metrics)`
over crate-specific metrics). A `lib/game` helper would have to take both
`scores` and `placings` as arguments - i.e. it would *be* `placings_log` - or
take closures, which is worse than the duplication. **Therefore: an identical
per-crate extract.** No file under `rust/lib/` is touched by this WP.

## 1. Problem

Every listed crate's `Gamer::command` (`rust/game/<crate>/src/lib.rs`) repeats
one ~10-15 line block - `if finished { build scores; build placings;
logs.push(placings_log(..)) }` - once per match arm. Copy counts per finding are
in the riders table (section 6): e F1 (major) 8, a F6 11, b F11 6, b F22 6,
b F33 5, c F6 5, e F13 3, d F21 / d F33 / f F7 / f F35 2 each.

**e F14** is the one non-mechanical item: age-of-war-2 *amplifies* the log.
`can_roll` is a deliberately preserved Go quirk that does not check finished
status, so the current player can keep issuing `roll` after the game ends and
each accepted command appends another placings log.

## 2. Why it's wrong

- Copy-paste of a finish epilogue per arm is a drift hazard, and the drift is
  already visible: greed-2's `Score` arm and 12 of starship-catan-1's 17 arms
  carry no epilogue at all, so any finish reachable through them would be
  announced by nothing.
- **e F1 is correct as written** - verified live: eight `Ok(ParseOutput)` arms
  in love-letter-2's `command` differ only in the `play_*` call.
- **e F14 is correct as written, and its recommendation is sound** - verified
  live: `command` gates on `self.is_finished()` (post-state), not on the
  false->true transition, so a post-finish `roll` re-appends the log.
- **Every other finding here is correct as written**; only the line numbers in
  the raw findings are stale.

## 3. Required end state

For each crate in the riders table, in `rust/game/<crate>/src/lib.rs`:

**3a. Add one private helper** on `impl Game`, beside the existing scoring
helpers, with the crate's own `scores` and `placings` expressions lifted
**verbatim** from the arms being deleted (do not "improve" them; confirm all
arms in that crate were byte-identical first):

```rust
fn finish_epilogue(&self, logs: &mut Vec<Log>) {
    logs.push(placings_log(&placings, Some(&scores)));
}
```

**3b. Hoist to a single gated call site** in `Gamer::command`. Capture
`let was_finished = self.is_finished();` (rtta: `self.finished`) before dispatch
and apply the false->true transition gate - this is e F14's fix, and a no-op in
every crate whose `command_parser` already returns `None` once finished.
Restructure the `match` so each arm yields its per-arm data and **one** tail
appends the epilogue and builds the response:

- Arms that build `CommandResponse` inline (all crates except the three below):
  each arm yields `(logs, can_undo, remaining)`; the tail appends the epilogue
  to `logs` and constructs the single `CommandResponse`. **Per-arm `can_undo`
  values must be preserved exactly** - texas-holdem-2's `Raise` is the only
  `true` in this package.
- Arms that yield a `CommandResponse` from a sub-command
  (roll-through-the-ages-2, age-of-war-2): each arm yields `resp`; the tail does
  `if !was_finished && <finished> { self.finish_epilogue(&mut resp.logs); }`.
- **acquire-1** already has the collapsed shape (`match output.value` dispatch
  plus one trailing `.map(|(logs, can_undo)| CommandResponse { .. })`). Move its
  lone `Done`-arm epilogue into that `.map`, gated as above. Behaviour is
  identical today - `end_turn`/`end()` is reachable only from `Done` - but it
  removes the drift hazard.
- **starship-catan-1** covers 5 of 17 arms today (`Gain`, `Upgrade`, `Found`,
  `Fight`, `Complete`); after the hoist all 17 are covered. Deliberate widening:
  `is_finished()` is `victory_points() >= 10`, so any arm that can raise VP now
  announces the result. Do not preserve the 5-arm subset.

The `Err(e)` arm keeps its existing shape and gets no epilogue.

## 4. Non-goals

- **`rust/game/red7-1` is not WP-08's** - no file in it may be touched.
- **lost-cities-1 / lost-cities-2: no code change** (closes the routed-in
  "double placings-log" item). Verified live: each has exactly one epilogue
  site, in the `Draw` arm, which is the only finishing path - nothing to dedup.
  The "double" is that a finished game announces the winner twice, via
  `end_round`'s `game_over_log()` and via `placings_log`. **Ruling: both stay.**
  WP-28 Task 4 deliberately rewrites `-2`'s `game_over_log()` and asserts its
  wording; removing either line contradicts that package and changes
  user-visible output with no finding behind it.
- No gameplay semantics change: do not add a finished-guard to `command_parser`
  or to age-of-war-2's `can_roll` (parked, Go-parity, test-covered), do not
  change scores, placings, points, `Status`, or any `RULES.md`.
- Do not touch the other findings in these crates (panics, unwraps, dead code,
  stats) - they belong to WP-13/14/15/19/21/22/23/25/29 and the `WP-09*` sweeps.
  modern-art-2 is excluded: WP-25 already hoists its epilogue itself.

## 5. Regression test cases

Add one test per crate, in that crate's existing test module. **The module name
is not uniform across these crates** - it is `mod test` in some and `mod tests`
in others; the riders table gives the correct one per crate. Extend an existing
game-to-completion test where one exists rather than building a new fixture.

- Drive a game to completion through `command()` and assert the returned logs
  contain **exactly one** log whose rendered text matches the placings log, and
  that it is the last log. This must hold for at least two different finishing
  commands where the crate has more than one finishing arm.
- Assert the finishing response's `can_undo` is unchanged from today's value
  (texas-holdem-2: also assert a non-finishing `Raise` still returns `true`),
  and that `status()` still returns the same `Status::Finished { placings }`.
- Assert a non-finishing command's logs contain no placings log.
- **age-of-war-2 (e F14)**: after the game is finished, issue a further `roll`
  through `command()` and assert the response contains **zero** placings logs.

## 6. Riders

All twelve findings are the same mechanical change; this table is the work list.
`Arms` is the count of `Command::` match arms in `command()` (approximate,
verify before editing).

| File (`rust/game/<crate>/src/lib.rs`) | Fix | Arms | Test module | Test |
|---|---|---|---|---|
| roll-through-the-ages-2 (a F6) | extract `finish_epilogue`, hoist; predicate is the `self.finished` field | 11 | `mod test` | y |
| love-letter-2 (e F1, major) | extract + hoist | 8 | `mod test` | y |
| seven-wonders-1 (b F11) | extract + hoist; placings is a local `gen_placings` over vp/coins | 6 | `mod tests` | y |
| alhambra-1 (b F22) | extract + hoist | 6 | `mod tests` | y |
| splendor-2 (b F33) | extract + hoist | 5 | `mod tests` | y |
| texas-holdem-2 (c F6) | extract + hoist; preserve `Raise`'s `can_undo: true` | 5 | `mod tests` | y |
| age-of-war-2 (e F13, e F14) | extract + hoist into `resp.logs`; transition gate is the F14 fix | 3 | `mod test` | y |
| jaipur-2 (d F21) | extract + hoist; keep the `(0..2)` / `NUM_PLAYERS` mix verbatim | 2 | `mod tests` | y |
| sushi-go-2 (d F33) | extract + hoist | 2 | `mod test` | y |
| zombie-dice-2 (f F7) | extract + hoist | 2 | `mod test` | y |
| greed-2 (f F35) | extract + hoist; `Score` arm gains coverage | 3 | `mod test` | y |
| acquire-1 (routed in) | move the `Done` epilogue into the existing trailing `.map`, gated | 9 | `mod tests` | n |
| starship-catan-1 (routed in) | extract + hoist; coverage widens 5 arms -> 17 | 17 | `mod tests` | y |
