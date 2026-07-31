# Mechanical Sweeps — rust/game/* — 2026-07-30

Method: `rg -n` searches across `/home/beefsack/Development/brdgme/rust/game/`,
followed by manual context checks (surrounding `impl Gamer` block boundaries,
struct-literal field values, test bodies) via `awk`/`sed` on the hit lines.
No tests were run; no source was modified. 28 crate directories exist under
`rust/game/`.

## Sweep 1 — `fn validate` override of `Gamer::validate`

Command: `rg -n "fn validate" rust/game/`, then for each hit confirmed no
top-level (column-0) `}` appears between the nearest preceding `impl Gamer`
line and the `fn validate` line (i.e. it is not a closed-off impl before the
`fn validate`).

| Crate | Overrides `validate`? | File:line |
|---|---|---|
| acquire-1 | no | - |
| age-of-war-2 | yes | rust/game/age-of-war-2/src/lib.rs:437 |
| alhambra-1 | no | - |
| battleship-2 | yes | rust/game/battleship-2/src/lib.rs:425 |
| category-5-2 | yes | rust/game/category-5-2/src/lib.rs:378 |
| cathedral-2 | no | - |
| farkle-2 | yes | rust/game/farkle-2/src/lib.rs:481 |
| for-sale-2 | yes | rust/game/for-sale-2/src/lib.rs:400 |
| greed-2 | yes | rust/game/greed-2/src/lib.rs:418 |
| hanamikoji-1 | yes | rust/game/hanamikoji-1/src/lib.rs:673 |
| jaipur-2 | no | - |
| liars-dice-2 | yes | rust/game/liars-dice-2/src/lib.rs:252 |
| lords-of-vegas-1 | no | - |
| lost-cities-1 | no | - |
| lost-cities-2 | yes | rust/game/lost-cities-2/src/lib.rs:550 |
| love-letter-2 | yes | rust/game/love-letter-2/src/lib.rs:809 |
| modern-art-2 | yes | rust/game/modern-art-2/src/lib.rs:742 |
| no-thanks-2 | yes | rust/game/no-thanks-2/src/lib.rs:236 |
| red7-1 | yes | rust/game/red7-1/src/lib.rs:562 |
| roll-through-the-ages-2 | no | - |
| seven-wonders-1 | no | - |
| splendor-2 | no | - |
| starship-catan-1 | no | - |
| sushi-go-2 | no | - |
| sushizock-2 | no | - |
| texas-holdem-2 | no | - |
| tic-tac-toe-2 | yes | rust/game/tic-tac-toe-2/src/lib.rs:190 |
| zombie-dice-2 | yes | rust/game/zombie-dice-2/src/lib.rs:453 |

15 yes / 13 no.

Note: two `fn validate` occurrences found by the raw `rg` search are test
functions, not trait overrides, and are excluded from the table above:
- `rust/game/tic-tac-toe-2/src/lib.rs:694` — `fn validate_rejects_inconsistent_state()` (test)
- `rust/game/lost-cities-2/src/lib.rs:943` — `fn validate_works()` (test)
- `rust/game/modern-art-2/src/lib.rs:1324` — `fn validate_catches_inconsistent_state()` (test)
- `rust/game/age-of-war-2/src/lib.rs:839` — `fn validate_catches_inconsistent_state()` (test)
- `rust/game/love-letter-2/src/lib.rs:1168` — `fn validate_rejects_inconsistent_state()` (test)

(These crates already have a genuine override elsewhere, listed above; the
test names simply overlap the search string `fn validate`.)

No `fn validate` was found outside an `impl Gamer` block in any crate.

## Sweep 2 — `Status::Finished` construction with empty `stats`

Command: `rg -n "Status::Finished" rust/game/`, filtered to bare
`Status::Finished {` struct-literal constructions (as opposed to
`Status::Finished { placings, .. } => ...` match-pattern usages in tests),
then inspected the `stats:` field of each.

### Empty-stats hits (`stats: vec![]`)

| File:line | Crate |
|---|---|
| rust/game/seven-wonders-1/src/lib.rs:902 | seven-wonders-1 |
| rust/game/starship-catan-1/src/lib.rs:1999 | starship-catan-1 |
| rust/game/sushizock-2/src/lib.rs:677 | sushizock-2 |
| rust/game/alhambra-1/src/lib.rs:920 | alhambra-1 |
| rust/game/liars-dice-2/src/lib.rs:280 | liars-dice-2 |
| rust/game/red7-1/src/lib.rs:422 | red7-1 |
| rust/game/jaipur-2/src/lib.rs:707 | jaipur-2 |
| rust/game/farkle-2/src/lib.rs:357 | farkle-2 |
| rust/game/sushi-go-2/src/lib.rs:770 | sushi-go-2 |
| rust/game/greed-2/src/lib.rs:445 | greed-2 |
| rust/game/greed-2/src/lib.rs:813 | greed-2 |
| rust/game/tic-tac-toe-2/src/lib.rs:262 | tic-tac-toe-2 |
| rust/game/tic-tac-toe-2/src/lib.rs:515 | tic-tac-toe-2 |
| rust/game/tic-tac-toe-2/src/lib.rs:530 | tic-tac-toe-2 |
| rust/game/for-sale-2/src/lib.rs:430 | for-sale-2 |
| rust/game/zombie-dice-2/src/lib.rs:481 | zombie-dice-2 |
| rust/game/cathedral-2/src/lib.rs:570 | cathedral-2 |
| rust/game/battleship-2/src/lib.rs:453 | battleship-2 |
| rust/game/no-thanks-2/src/lib.rs:259 | no-thanks-2 |
| rust/game/age-of-war-2/src/lib.rs:468 | age-of-war-2 |
| rust/game/category-5-2/src/lib.rs:442 | category-5-2 |
| rust/game/roll-through-the-ages-2/src/lib.rs:1633 | roll-through-the-ages-2 |
| rust/game/acquire-1/src/lib.rs:206 | acquire-1 |
| rust/game/lords-of-vegas-1/src/lib.rs:201 | lords-of-vegas-1 |

No `stats: Vec::new()` or `stats: Default::default()` forms were found; all
empty cases use the `stats: vec![]` literal.

### Crates whose `Status::Finished` construction populates `stats` (excluded above)

| Crate | File:line | Expression |
|---|---|---|
| hanamikoji-1 | rust/game/hanamikoji-1/src/lib.rs:736 | `stats: self.finished_stats()` |
| love-letter-2 | rust/game/love-letter-2/src/lib.rs:669 | `stats: vec![Default::default(); self.players]` |
| texas-holdem-2 | rust/game/texas-holdem-2/src/lib.rs:775 | `stats: vec![HashMap::new(); self.players]` |
| lost-cities-2 | rust/game/lost-cities-2/src/lib.rs:583 | `stats: (0..self.players).map(\|p\| self.player_stats(p)).collect()` |
| splendor-2 | rust/game/splendor-2/src/lib.rs:583 | `stats: vec![Default::default(); self.players]` |
| lost-cities-1 | rust/game/lost-cities-1/src/lib.rs:514 | `stats: vec![self.player_stats(0), self.player_stats(1)]` |
| modern-art-2 | rust/game/modern-art-2/src/lib.rs:619 | `stats: vec![HashMap::new(); self.players]` |

### Total `Status::Finished` construction count per crate (context)

Bare struct-literal constructions only (excludes match-pattern uses in tests):

| Crate | Constructions |
|---|---|
| acquire-1 | 1 |
| age-of-war-2 | 1 |
| alhambra-1 | 1 |
| battleship-2 | 1 |
| category-5-2 | 1 |
| cathedral-2 | 1 |
| farkle-2 | 1 |
| for-sale-2 | 1 |
| greed-2 | 2 |
| hanamikoji-1 | 1 |
| jaipur-2 | 1 |
| liars-dice-2 | 1 |
| lords-of-vegas-1 | 1 |
| lost-cities-1 | 1 |
| lost-cities-2 | 1 |
| love-letter-2 | 1 |
| modern-art-2 | 1 |
| no-thanks-2 | 1 |
| red7-1 | 1 |
| roll-through-the-ages-2 | 1 |
| seven-wonders-1 | 1 |
| splendor-2 | 1 |
| starship-catan-1 | 1 |
| sushi-go-2 | 1 |
| sushizock-2 | 1 |
| texas-holdem-2 | 1 |
| tic-tac-toe-2 | 3 |
| zombie-dice-2 | 1 |

## Sweep 3 — `pub_state` redaction: structure and tests

Method: `rg -n "fn pub_state|PubState|PublicState"` per crate to confirm a
distinct `PubState` type (all 28 crates define `pub struct PubState { ... }`
and `type PubState = PubState;` in their `impl Gamer`), then
`rg -n "fn \w*pub_state\w*\(\)"` plus manual reading of each matching test
body to judge whether it actually *asserts* redaction (something hidden is
absent/limited in `PubState`) versus merely calling `pub_state()` for
rendering or field-capture purposes.

| Crate | (a) distinct PubState type | (b) test asserting redaction | Evidence (file:line) |
|---|---|---|---|
| acquire-1 | yes | no | rust/game/acquire-1/src/lib.rs:1500 `game_can_end_matches_pub_state_can_end` (not a redaction assertion) |
| age-of-war-2 | yes | no | rust/game/age-of-war-2/src/lib.rs:812 `pub_state_carries_full_public_info` (asserts everything is public, not that anything is hidden — no hidden info in this game) |
| alhambra-1 | yes | yes | rust/game/alhambra-1/src/lib.rs:1366 `pub_state_does_not_leak_hidden_info` |
| battleship-2 | yes | yes | rust/game/battleship-2/src/lib.rs:966 `test_pub_state_redacts_ships` |
| category-5-2 | yes | no | rust/game/category-5-2/src/lib.rs:869 `test_pub_state_captures_rendered_fields` (only compares public fields, no assertion of hidden-field omission) |
| cathedral-2 | yes | no | rust/game/cathedral-2/src/lib.rs:1297 `pub_state_and_player_state_render_identically` (no hidden info in this game) |
| farkle-2 | yes | no | rust/game/farkle-2/src/lib.rs:660 `test_finished_pub_state_clears_turn_fields` (post-finish field reset, not redaction) |
| for-sale-2 | yes | yes | rust/game/for-sale-2/src/lib.rs:869 `test_pub_state_redacts_hands_and_cheques` |
| greed-2 | yes | no | no `pub_state()` call anywhere in `#[test]` fns |
| hanamikoji-1 | yes | yes | rust/game/hanamikoji-1/src/lib.rs:1103 `test_redaction` |
| jaipur-2 | yes | yes | rust/game/jaipur-2/src/lib.rs:1244 `pub_state_does_not_leak_hand_contents` |
| liars-dice-2 | yes | yes | rust/game/liars-dice-2/src/lib.rs:461 `pub_state_redacts_dice_player_state_reveals_own` |
| lords-of-vegas-1 | yes | no | no `pub_state()` call anywhere in `#[test]` fns |
| lost-cities-1 | yes | no | no `pub_state()` call anywhere in `#[test]` fns |
| lost-cities-2 | yes | no | no `pub_state()` call anywhere in `#[test]` fns |
| love-letter-2 | yes | yes | rust/game/love-letter-2/src/lib.rs:1152 `pub_state_does_not_leak_hidden_info` |
| modern-art-2 | yes | yes | rust/game/modern-art-2/src/lib.rs:1146 `test_pub_state_hides_sealed_bids_and_money` |
| no-thanks-2 | yes | yes | rust/game/no-thanks-2/src/lib.rs:557 `test_pub_state_chips_hidden_until_finished` |
| red7-1 | yes | yes | rust/game/red7-1/src/lib.rs:810 `pub_state_does_not_leak_hidden_info` |
| roll-through-the-ages-2 | yes | no | no `pub_state()` call anywhere in `#[test]` fns |
| seven-wonders-1 | yes | yes | rust/game/seven-wonders-1/src/lib.rs:1729 `test_pub_state_does_not_leak_hidden_info` |
| splendor-2 | yes | yes | rust/game/splendor-2/src/lib.rs:1237 `test_pub_state_reserve_counts_no_content` |
| starship-catan-1 | yes | yes | rust/game/starship-catan-1/src/lib.rs:2222 `pub_state_does_not_leak_hidden_info` |
| sushi-go-2 | yes | yes | rust/game/sushi-go-2/src/lib.rs:1445 `test_pub_state_redacts_hands` |
| sushizock-2 | yes | no | rust/game/sushizock-2/src/lib.rs:1683 `test_pub_state_no_hidden_info` (confirms nothing is hidden by design, not a redaction assertion) |
| texas-holdem-2 | yes | yes | rust/game/texas-holdem-2/src/lib.rs:1265 `pub_state_does_not_leak_hands_or_deck` |
| tic-tac-toe-2 | yes | no | rust/game/tic-tac-toe-2/src/lib.rs:658 `states_capture_visible_game_data` (no hidden info in this game; test only checks field capture) |
| zombie-dice-2 | yes | no | rust/game/zombie-dice-2/src/lib.rs:1029 `test_pub_state_captures_rendered_fields` (field capture only, no omission assertion) |

15 yes / 13 no on (b).

Of the 13 "no" crates, 7 are documented (in `DATA_DOCS.md`/code comments) as
having no hidden per-player information at all (age-of-war-2, cathedral-2,
greed-2, roll-through-the-ages-2, sushizock-2, tic-tac-toe-2, and — per
in-file comments — farkle-2 for the turn-field-only test), so the absence of
a redaction test may be lower-risk there than for the remainder
(acquire-1, category-5-2, lords-of-vegas-1, lost-cities-1, lost-cities-2)
which do carry per-player hidden state (hands/tiles/etc.) but have no test
asserting it stays out of `PubState`.
