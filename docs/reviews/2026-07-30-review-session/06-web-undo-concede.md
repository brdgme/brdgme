# Unit 06 - Web domain: undo/concede integrity

- **Maps to**: WP-40 (D-03/D-04)
- **Commit reviewed**: `9ba3736b` (91 files, 3838+/175-)
- **Findings numbered from**: F-111

Files reviewed in final form: `rust/web/src/db/game_write.rs`,
`rust/web/src/db/rating.rs`, `rust/web/src/db/bots.rs`,
`rust/web/src/game/server_fns.rs`, `rust/web/src/email/commands.rs`,
`docs/CODING.md`, `docs/ARCHITECTURE.md`.

Spec recovered from `868094a6:.../specs/WP-40-undo-concede-toctou-ratings-integrity.md`
(486 lines, 7 tasks, 9-row regression-test plan). Decisions D-3/D-4 = option A.

## Progress

Attempt 4 (resumed). Salvaged from attempt 3: F-111..F-115 confirmed and flushed
(concede transaction boundary, concede idempotency, `left_at` snapshot check,
trimmed test assertions, `concede_game` 2-player predicate).

Remaining when attempt 4 started:
- The other WP-40 tasks (1, 3, 4, 5, 7) - notably the rating-corruption critical
  and `rating.rs` / `game_write.rs::undo_game` end state.
- The 91-file breadth of `9ba3736b` (breakdown gotcha: "confirm the shared core is
  actually used everywhere - no game crate quietly kept its own bypass").
- `email/commands.rs::run_concede`, `docs/CODING.md` / `docs/ARCHITECTURE.md`
  D-03/D-04 records.
- `## Verified good` and `## Coverage gaps` sections.

Attempt 4 added F-116, F-117, F-118 from reading the final code of
`game_write.rs` (undo/concede/end_game), `rating.rs` and `game/placing.rs`.

Spec + commit shape recovered (worker 1, cached at
`scratchpad/w1-wp40.md` in this session's tmp dir). **The breakdown's Unit 06
gotcha is based on a false premise**: `9ba3736b` touches **zero** `rust/game/*`
files. Its 91 files are 83 generated sqlx query-cache JSONs (~73% of the
insertions), 4 docs, and 4 Rust sources (`db/game_write.rs` +549/-13,
`game/server_fns.rs` +142/-67, `db/rating.rs` +70/-7, `email/commands.rs`
+12/-84). There is no shared-core extraction across game crates to verify.

Attempt 4 added F-116..F-119 plus the `## Verified good`, `## Coverage gaps` and
`## Carry-forwards` sections. Tasks 1, 2, 3 (cores), 5 and D-3 verified good
against the recovered spec by direct reading of the final code.

**UNIT COMPLETE.** All verification landed. Findings F-111..F-120 (2 attempts'
work, all confirmed against the final code). Tasks 1-7 all checked individually
against the recovered spec; Tasks 3 (`can_undo`), 4 (email collapse), 7 (both doc
blocks, byte-identical to the spec's prescribed text) and the full 8-test roster
all verified present and correct. The only spec-prescribed test name absent is
one the spec never asked for (`concede_game_replace_rejects_stale_updated_at`).

## Findings

### F-111 (High) - `concede_game_replace` inserts its `game_bots` row on the pool *before* opening the transaction, so every rejected concede leaks an orphan bot row - and the spec's assertion that would have caught it was dropped from the test

`rust/web/src/db/game_write.rs:394-399`, `rust/web/src/db/bots.rs:76-98`,
`rust/web/src/db/game_write.rs:2178-2242`.

```rust
let bot = pick_replacement_bot(pool, game_id)      // :394  <-- autocommit, on &PgPool
    .await?
    .ok_or_else(|| anyhow::anyhow!("no replacement bot configured"))?;

let mut tx = pool.begin().await?;                  // :398
claim_unfinished_game_tx(&mut tx, game_id, expected_updated_at).await?;  // :399
```

`pick_replacement_bot` is `SELECT name FROM bots ... ` then
`INSERT INTO game_bots ... RETURNING` as **two separate autocommit statements on
the pool** (`bots.rs:80-96`). It runs *before* `pool.begin()`, so the inserted
`game_bots` row is committed and is not covered by the transaction the claim
guards. Every path that makes the claim fail - `GameAlreadyFinished`,
`StaleStateConflict`, "Game not found" - and every later failure inside the
transaction, rolls back the `game_players` swap and the log line but leaves the
`game_bots` row behind permanently. This is the carry-forward Unit 05b flagged,
and it is worse than a signature nit: the guard WP-40 added is precisely what
makes the leak routine rather than rare.

Why it matters: `game_bots` rows are the game's bot roster. An orphan row is a
bot that belongs to the game but is not attached to any `game_players` slot. It
is user-visible wherever the roster is read, it accumulates one row per rejected
concede attempt (a user who retries a concede on a game that just finished mints
one each time), and it blocks nothing, so nothing surfaces the inconsistency.

**Compounding (pattern 4b, fifth confirmed instance).** The spec's own
regression-test row for this case reads:

> `concede_game_replace_rejects_finished_game` ... `Err` → `GameAlreadyFinished`;
> **no `game_bots` row added**, `left_at` still NULL

The landed test (`game_write.rs:2228-2241`) asserts the error type and
`left_at.is_none()` and **contains no `game_bots` assertion at all**. The one
acceptance criterion that would have failed is the one that is missing. The test
now certifies the case as covered.

**Escalation confirmed after the initial write-up.** `game_bots` carries
`UNIQUE (game_id, name)` (`rust/web/migrations/003:15`, still in force after
013), and `pick_replacement_bot` INSERTs with **no `ON CONFLICT`**
(`bots.rs:88-93`). So the orphan row is not merely cosmetic: it poisons the game.
Deployments normally configure a single `can_replace_humans` bot, so
`ORDER BY random() LIMIT 1` returns the same name every time. The sequence is:

1. Concede is attempted, the claim fails (game just finished, or a stale
   `updated_at`). The `game_bots` row is already committed. Orphan created.
2. The user retries. `pick_replacement_bot` picks the same name, the INSERT
   violates `UNIQUE (game_id, name)`, and the error is a bare `sqlx::Error` -
   neither `GameAlreadyFinished` nor `StaleStateConflict` - so
   `conflict_or_internal` (`server_fns.rs:833-843`) redacts it to
   `INTERNAL_ERROR_MESSAGE`.
3. **Concede-with-replacement is now permanently impossible for that game**, with
   a generic internal error and no operator-visible cause.

This is the same terminal failure shape as F-115, reached by a second route, and
it is caused directly by the ordering WP-40 introduced. The missing
`game_bots` assertion in the test (below) is the assertion that would have caught
step 1.

Remediation: change `pick_replacement_bot` to take `&mut sqlx::PgConnection`
(matching `claim_unfinished_game_tx`, `apply_rating_changes` and
`write_ranked_placings`, which all already do) and call it *after* the claim,
inside the transaction. Then restore the spec's
`SELECT count(*) FROM game_bots WHERE game_id = $1` assertion to the test. The
only other caller is `concede_core`'s `replacement_bot_available` probe, which is
a different function and unaffected.

### F-112 (High) - `concede_game_replace` never touches the `games` row, so its `updated_at` claim can never fail against its own effect: a duplicated concede swaps in a second bot and writes a second public log line

`rust/web/src/db/game_write.rs:387-426`, `rust/web/src/game/server_fns.rs:945-963`.

`concede_game_replace` writes only `game_players` (bot id, `left_at`,
`undo_game_state = NULL`, `turn_reminder_sent_at`) and a `game_logs` row. The
spec calls this out as the reason the claim helper is the *only* available guard
(`concede_game_replace` "has no UPDATE to hang a WHERE clause on"). But
`games.updated_at` is maintained by the `update_games_updated_at` trigger on the
`games` table only - the sibling trigger on `game_players` bumps
`game_players.updated_at`, not the parent (this is exactly what
`update_updated_at_trigger_maintains_games_and_game_players`,
`game_write.rs:1684`, pins). So a successful `concede_game_replace` leaves
`games.updated_at` **unchanged**.

Consequence: the claim is not an idempotency guard for this function. Replay the
same request - a double-clicked concede button, two deliveries of the same
inbound email command (`run_concede` has no dedup), a retried server fn - with
the same `ge.game.updated_at`, and the second call finds `is_finished = false`
and `updated_at == expected_updated_at`, passes the claim, and swaps in a
*second* replacement bot over the first, overwrites `left_at`, and appends a
second `"X conceded (replaced by bot Y)."` public log line. Combined with F-111
the game accumulates two `game_bots` rows, one of them orphaned.

`undo_game` got the exact defence this needs - the spec's Task 2.3 re-verifies
`undo_game_state IS NOT NULL` inside the transaction, with the stated rationale
"without this a doubled undo request replays a stale snapshot even though the row
lock was clean" (`game_write.rs:546-556`). The identical reasoning applies to
`concede_game_replace` and no equivalent check was written, so the package
hardened one of the two functions that needed the same treatment (pattern 2).

Remediation: inside the transaction, after the claim, make the `game_players`
UPDATE conditional and check the row count:
`... WHERE id = $2 AND left_at IS NULL AND game_bot_id IS NULL`, 0 rows affected
→ `StaleStateConflict`. That also closes F-113.

### F-113 (Medium) - `concede_core`'s `left_at` check is a pool-snapshot check with no in-transaction counterpart - the precise defect ("a check against a snapshot is not a guard") WP-40 was written to eliminate

`rust/web/src/game/server_fns.rs:945-947`.

```rust
if player.game_player.left_at.is_some() {
    return Err(ServerFnError::new("You have already left this game"));
}
```

`ge` was read from the pool at `:923`; between that read and the write there is a
`replacement_bot_available` round trip (`:950`) and, on the replace branch, a
second pool round trip inside `pick_replacement_bot`. Neither
`claim_unfinished_game_tx` nor `concede_game_replace` re-checks `left_at`, so
"already left" is enforced only against a stale snapshot. The spec's own root-cause
section (Defect 2) states the rule this violates verbatim: *"A check against a
snapshot is not a guard; it only narrows the window."* The package applied that
rule to `is_finished` and `updated_at` and left `left_at` on the old footing.

Same for `count_active_humans` (`:819-824`, also derived from `left_at`): the
branch choice between `concede_game_replace`, `concede_game` and the refusal is
made from the snapshot and never revalidated.

Remediation: as F-112 - the `AND left_at IS NULL` predicate on the
`game_players` UPDATE, 0 rows → `StaleStateConflict`. `concede_game` needs the
same predicate on its per-player writes or an equivalent in-transaction re-read.

### F-114 (Medium) - all seven new guard tests exist with the spec's names and error-type assertions, but three of them dropped the specific "nothing was destroyed" assertion the spec identified as the point of the test (pattern 4b)

`rust/web/src/db/game_write.rs:1816-2282`.

The spec's regression-test plan is a table of nine rows, each naming both the
error type *and* the state assertions. All seven guard tests landed with the
prescribed names and the correct `downcast_ref` assertions. The state assertions
were selectively trimmed:

| Test | Spec required | Landed test asserts |
|---|---|---|
| `undo_game_rejects_stale_updated_at` (`:1889`) | `game_state` unchanged; **"both players' `undo_game_state` still exactly as the second move left them (this is the 'destroys nothing' assertion wd F15 is about)"** | `game_state == "state_2"` only (`:1967`). No `undo_game_state` assertion. |
| `undo_game_rejects_finished_game` (`:1816`) | `game_state`, `is_finished`, `finished_at`, **both `place`** and both `rating_change` unchanged | `game_state`, `is_finished`, `finished_at`, `rating_change.is_some()` (`:1877-1882`). No `place` assertion; `is_some()` is not "unchanged". |
| `concede_game_replace_rejects_finished_game` (`:2178`) | `Err`; **no `game_bots` row added**; `left_at` still NULL | error type + `left_at.is_none()` (`:2241`). No `game_bots` assertion - see **F-111**, that assertion would have failed. |
| `concede_game_requires_two_players` | `Err` (Task 6); **no places written** | 3 humans, `is_err()`, `place == None` for every player. Correct as far as the row goes - but it does not assert `is_finished` stayed false, and `concede_game` runs `UPDATE games SET is_finished = true` (`:330-336`) *before* the `players.len() != 2` check (`:350`). The rollback is therefore load-bearing for this test's own scenario and is the one thing it does not check. |

The `undo_game_rejects_stale_updated_at` case is the notable one on its own terms:
the spec annotated that exact assertion as *the* thing wd F15 was about, and it is
the one line that is gone. The underlying behaviour happens to be correct here
(`claim_unfinished_game_tx` returns before any write and the transaction is
dropped), so this is a coverage defect rather than a hidden bug - but it is
indistinguishable from F-111, where the same trimming hides a live defect. Note
what the pattern costs: a reviewer checking "did WP-40's test plan land" sees
seven correctly named tests and stops.

Remediation: restore the three dropped assertions. The `game_bots` one first - it
fails today.

### F-115 (Medium) - Task 6's error is documented as unreachable on the strength of a caller gate that does not imply it; via in-game elimination a 3-player game reaches `concede_game` and concede becomes permanently impossible with a redacted "internal error"

`rust/web/src/db/game_write.rs:348-355`, `rust/web/src/game/server_fns.rs:819-824,
949, 964-978`.

Task 6 replaced `debug_assert!(players.len() == 2)` with a real error, on the
spec's stated reasoning: *"Both callers already gate on `active_humans == 2`
before choosing this branch, so this is unreachable in practice."* The gate is
`count_active_humans` (`server_fns.rs:819-824`), which counts
`user_id.is_some() && left_at.is_none()`. `concede_game`'s new check counts **all**
`game_players` rows for the game (`game_write.rs:341-344` - no `left_at` or
`user_id` filter). The two are equal only when every row in the game is an active
human.

They diverge on a path the codebase already exercises: `update_game_command_success`
stamps `left_at` on elimination (pinned by `elimination_sets_left_at_once`,
`game_write.rs:1291`). So a 3-player human game in which one player is eliminated
by gameplay has `active_humans == 2` and `players.len() == 3`. With no replacement
bot configured, `concede_core` takes the `active_humans == 2` branch (`:964`),
`concede_game` returns the bare `anyhow!("concede_game requires exactly 2
players, found 3")`, and because that is not one of the two distinguishable
types, `conflict_or_internal` (`:833-843`) falls through to `internal(context)`,
which redacts it to `INTERNAL_ERROR_MESSAGE`. The user sees a generic internal
error, retries forever, and concede is permanently unavailable for that game. It
also fires in a 3-player game with one already-replaced human whose replacement
bot has since been disabled.

Task 6 is still an improvement over the `debug_assert!` - a redacted error beats
silently mis-placing and rating a 3+-player game. The finding is that the gate the
spec relied on to make it unreachable is the wrong predicate, so the case is
reachable and lands in the worst error class.

Remediation (F-115): change the branch condition in `concede_core` from
`active_humans == 2` to the predicate `concede_game` actually requires (total
`game_players` for the game == 2), and give the 3+-player no-bot case the existing
explicit refusal text rather than falling into `concede_game`. Alternatively
generalise `concede_game` to place the conceder last among active players and the
non-conceders by their existing order - but that is a behaviour change and belongs
in a spec, not a patch.

### F-116 (High) - `undo_game` sets `left_at` when a player becomes eliminated but never clears it when the undo un-eliminates them, so an undone elimination permanently marks the player a "leaver" - and `compute_ranked_placings` ranks every leaver below every survivor, so the undo the player was granted costs them the game's rating

`rust/web/src/db/game_write.rs:584-598`, `rust/web/src/game/placing.rs:15-44`,
`rust/web/src/db/rating.rs:26-30,162-165`.

`undo_game`'s per-player UPDATE is:

```sql
SET is_turn = $1, is_eliminated = $2, place = $3, undo_game_state = NULL,
    turn_reminder_sent_at = NULL,
    left_at = CASE WHEN is_eliminated = false AND $2 = true
                   THEN NOW() ELSE left_at END
```

The CASE is copied from `update_game_command_success` (`:743-744`), where it is
correct: that function only ever moves a player *into* elimination. `undo_game`
is the one function in the codebase that moves a player *out* of it - it writes
`is_eliminated = $2` unconditionally from the restored state - and the CASE has
no arm for that direction. Undoing the command that eliminated a player restores
`is_eliminated = false` and leaves `left_at` at the timestamp the elimination
stamped.

Note the asymmetry this creates with `is_eliminated`, which *is* correctly
rewound in the same statement. The two columns are set from the same restored
status and are meant to agree; after an undo across an elimination boundary they
disagree permanently, because nothing else in the codebase ever clears `left_at`
(no `left_at = NULL` write exists anywhere).

Why it matters - three consequences, worst first:

1. **Rating.** `write_ranked_placings` classifies by `left_at.is_some()` alone
   (`placing.rs:17,28`) and places *all* leavers after *all* survivors regardless
   of their `place`. `apply_rating_changes` then scores pairwise on
   `ranked_placing.or(place)` (`rating.rs:164`). So a player who was eliminated,
   undid it, and went on to win the game is rated as if they finished last. This
   is rating corruption arriving through the undo path - the exact class WP-40
   was chartered to close (D-03/D-04), reached by a different route than the
   `undo_game`-on-a-finished-game case the package did fix.
2. **Concede.** `count_active_humans` (`server_fns.rs:819-824`) filters on
   `left_at.is_none()`, so the un-eliminated player stops counting as active.
   In a 3-player game this drops `active_humans` to 2 and steers `concede_core`
   into `concede_game`, which then fails on its 3-row check - F-115's failure
   mode, reached without any player actually leaving.
3. The player is presented as having left the game wherever `left_at` is
   rendered, while still taking turns.

Remediation: make the CASE two-directional -
`left_at = CASE WHEN $2 = true AND is_eliminated = false THEN NOW()
                WHEN $2 = false AND is_eliminated = true THEN NULL
                ELSE left_at END`
- but only for eliminations. A `left_at` set by concede/replacement must not be
cleared, and `undo_game` cannot currently distinguish the two causes from the
`game_players` row alone. The safe form is to guard the clear on the player
having no `game_bot_id` and the row not having been conceded, or to split the
"eliminated" and "left" concepts into separate columns (the cleaner fix - they
are conflated today, which is the root cause).

**Provenance, verified against the diff - and it makes the finding sharper, not
weaker.** The `left_at = CASE` clause in `undo_game` is byte-identical before and
after `9ba3736b`; it does not appear in the diff at all. It was introduced by
`7388923` (the elimination-stamp change, #47) and relocated by `4d31f6e`
(WP-82's module split). So WP-40 did not write this bug.

What WP-40 did do is edit **the sibling copy of the same expression**. In
`update_game_command_success` the identical CASE gained a new `AND NOT $9`
conjunct in this very commit (`:743-744`), to stop the finishing command from
stamping `left_at`. The author was therefore looking directly at this expression,
in this package, and hardened one of its two occurrences. This is pattern 2 -
inconsistent hardening within a single file - and the un-hardened copy is the one
in the function whose name is on the package (`undo_game`), in a package whose
charter is "ratings integrity".

Confirmed by sweep: `rg` over all of `rust/` finds **no** code path anywhere that
sets `left_at` back to NULL. The column is write-once in practice.

### F-117 (Medium) - `concede_game` finishes a game and rates it without calling `write_ranked_placings`, so every conceded game has `ranked_placing` NULL while every other finish path populates it - and the unit test for the concede case feeds `compute_ranked_placings` an input shape the concede path never produces (pattern 4f)

`rust/web/src/db/game_write.rs:356-381`, `rust/web/src/db/rating.rs:26-33`,
`rust/web/src/game/placing.rs:106-127`.

`write_ranked_placings`'s own doc comment states the contract: *"Must run in the
same transaction as the placings write and before `apply_rating_changes`."* Both
other finish paths obey it - `end_game` (`:468-469`) and
`update_game_command_success` (`:760-763`) call the pair. `concede_game`
(`:379`) calls `apply_rating_changes` alone. Result: a game finished by concede
gets `place` and `rating_change` but leaves `ranked_placing` NULL for every
player, permanently, while an identical game finished by play has it set.

For the 2-player concede the *rating* comes out the same, because
`apply_rating_changes` falls back with `ranked_placing.or(place)`. The defect is
in the persisted data, not the arithmetic: `ranked_placing` is the column
WP-40's own placing model introduced as the authoritative "how did this human
actually finish" value, and it is silently absent from exactly the finish path
that motivated inventing it.

Compounding: `placing.rs`'s `two_player_concede` test (`:106-127`) constructs
the conceder with `left_at: Some(ts(5))` and asserts the leaver ordering works.
`concede_game` **never sets `left_at`** on the conceder - only
`concede_game_replace` does, and that path does not finish the game and does not
call this code. So the one test named for the concede case models a state the
concede path cannot produce, and the state it does produce is never exercised
here at all. This is pattern 4f: the test is correct about the function and
wrong about the system.

Remediation: call `write_ranked_placings(&mut tx, game_id).await?;` immediately
before `apply_rating_changes` in `concede_game`, and rename/re-fixture the
`two_player_concede` test to the shape `concede_game` actually writes (conceder
`place = 2`, `left_at` NULL). Decide explicitly whether a conceder should carry
`left_at` - if yes, `concede_game` must set it and the test is right; if no, the
test is wrong. Today neither is true.

### F-118 (Low) - `undo_game` restores `game_state` and every status-derived column but not `game_players.points`, so points stay at their post-undone-move value until the next command; `end_game` orders placings by that column

`rust/web/src/db/game_write.rs:584-598` vs `:736-757`, `:440-445`.

`update_game_command_success` writes `points = $4` from the engine's
recomputed points. `undo_game`'s UPDATE omits the column entirely, so after an
undo the row still holds the points the undone move produced. Normally the next
command overwrites it, so the window is short. It is not always short: if the
undo is the last thing to happen in the game and it is then ended
administratively, `end_game` orders `ORDER BY points DESC NULLS LAST` (`:441`)
off the stale values and assigns `place` from that order - which feeds
`apply_rating_changes`.

Low rather than Medium because `end_game`-after-undo-with-no-intervening-command
is a narrow path, and `points` is not otherwise authoritative. Remediation:
either restore points in `undo_game` (the caller has the recomputed status; it
would need points threaded through the same way `update_game_command_success`
gets them), or document the column as advisory and stop ordering `end_game` by
it.

### F-119 (High) - `concede_game_replace` clears `is_turn` on the player it replaces without giving the turn to anyone, so conceding on your own turn wedges the game: no row has `is_turn = true`, `find_bot_turns` returns empty, and no bot is ever triggered

`rust/web/src/db/game_write.rs:401-410`, `rust/web/src/db/bots.rs:20-34`,
`rust/web/src/game/mod.rs:50-59`.

The swap statement is:

```sql
UPDATE game_players
   SET is_turn = false, game_bot_id = $1, left_at = NOW(),
       undo_game_state = NULL, turn_reminder_sent_at = NULL
 WHERE id = $2
```

`is_turn = false` is unconditional, and nothing else in the function or in
`concede_core` re-derives whose turn it is. `games.game_state` is untouched, so
the engine still says it is that position's turn - but the DB now says nobody's
turn is active.

The consequence chain is short and total:

- `broadcast_and_trigger` (`game/mod.rs:51-59`) is the shared post-commit
  epilogue; its bot half is `trigger_bot_turns`, which reads `find_bot_turns`.
- `find_bot_turns` (`bots.rs:20-34`) selects
  `WHERE gp.game_id = $1 AND gp.is_turn = true`. After the swap that returns
  **zero rows**, so the freshly installed replacement bot is never published a
  `bot.turn` and never plays.
- The remaining human cannot act either: it is not their position's turn in the
  game state.

The game is wedged. This is the common case, not an edge: a player conceding is
very often the player being waited on - that is usually *why* they concede.

Contrast the two neighbouring writers, which both get this right for their own
semantics: `concede_game` and `end_game` clear `is_turn` on everyone *and* finish
the game, so no turn is owed; `update_game_command_success` recomputes `is_turn`
per player from `status.whose_turn`. `concede_game_replace` is the only lifecycle
writer that clears a turn while leaving the game live, and it is the one the
spec's Task 2 singled out as structurally different ("writes `game_players` ...
without touching the `games` row at all"). The spec noticed the difference and
drew only the locking conclusion from it.

Why WP-40 is the right owner even though it did not introduce `is_turn = false`:
Task 2 rewrote this statement (adding `undo_game_state = NULL` and
`turn_reminder_sent_at = NULL` to it) and the package owns
`concede_game_replace`'s correctness. The `turn_reminder_sent_at = NULL`
addition makes it marginally worse - the turn-reminder sweep is the one mechanism
that might have surfaced a stalled turn to a human, and it gates on
`is_turn_at`/`turn_reminder_sent_at` for a row whose `is_turn` is now false.

Remediation: the replacement bot inherits the conceder's turn - the swap should
preserve `is_turn` rather than clear it (`is_turn` is already correct for that
position, since the engine's view did not change). Drop `is_turn = false` from
the statement. Then `find_bot_turns` picks the new bot up on the very next
`broadcast_and_trigger` and the game continues. Add a regression test asserting
that after `concede_game_replace` on a player whose `is_turn` was true,
`find_bot_turns(game_id)` returns exactly one row naming the new bot.

*Status: confirmed by reading the final code and the trigger path. Not yet
cross-checked against WP-38's bot-turn wedge-recovery sweep (Unit 05's) - if that
sweep re-derives status from the game service rather than from `is_turn`, it
would mask this in production without making the write correct. That sweep's gate
is the thing to check; see Coverage gaps.*

### F-120 (Medium) - `end_game` is the fourth game-lifecycle writer, it rates the game, and it was left with no `expected_updated_at` and no claim - so WP-40's own new `docs/CODING.md` rule is violated by the tree it was written into

`rust/web/src/db/game_write.rs:430-473`, `docs/CODING.md:653-674`,
`rust/web/src/email/commands.rs` (`run_end`).

Task 7's landed doc text (verbatim from the spec - confirmed byte-identical)
states the rule and then enumerates its subjects:

> Every mutating game-lifecycle function (`concede_game`, `concede_game_replace`,
> `undo_game`) takes `expected_updated_at` and opens its transaction with
> `claim_unfinished_game_tx` ...

The general rule the spec's Task 7 section states is broader - *"Every `db.rs`
function that reads game state and then writes it must take an
`expected_updated_at` and claim the row"* - with the stated review procedure
"reviewer greps for `UPDATE game` in db.rs". Run that grep and `end_game`
(`:430`) comes back. It:

- writes `UPDATE games SET is_finished = true, finished_at = NOW()` with **no**
  `updated_at` predicate and no claim (`:433-438`);
- then reads `game_players ORDER BY points DESC NULLS LAST` and assigns `place`
  from that read (`:440-458`);
- then calls `write_ranked_placings` **and `apply_rating_changes`** (`:468-469`).

So it is a read-then-write ending in an irreversible rating write, racing the
move path with no optimistic lock. A move landing between its `UPDATE games` and
its `SELECT ... ORDER BY points` is rated against whichever ordering wins. It is
reachable from two entry points - a server fn and the email `run_end` verb, which
Task 4 deliberately left alone (verified: `run_end` still calls `db::end_game`
directly and was untouched by `9ba3736b`). F-118's stale-`points` window feeds
straight into the same `ORDER BY`.

The spec non-goaled it explicitly ("`end_game` ... keep their current
signatures"), and the implementation obeyed. The finding is not that the
implementer disobeyed; it is that **the package shipped a doc rule and an
undocumented exemption from that rule in the same commit**. A reviewer running
the procedure the doc prescribes finds a violation on day one, and the
three-function enumeration reads as the complete list rather than as the subset
that happened to be in scope. Note the shape: this is the mirror image of pattern
4b - not a test edited down to agree with the code, but a rule scoped narrowly
enough that the code it does not cover becomes invisible.

Remediation: either extend the claim to `end_game` (same shape as `concede_game`;
both callers already hold `ge.game.updated_at`), or state the exemption in
`docs/CODING.md` beside the rule with its reason, so the grep procedure has a
known-answer list. The former is small and closes a live race on a rating write;
prefer it.

## Verified good

Checked against the recovered spec's acceptance criteria, reading the final code
(not the commit message). The core of WP-40 is well built; the findings above are
gaps at its edges, not a failed package.

- **Task 1 - `claim_unfinished_game_tx`** (`game_write.rs:633-651`). Matches the
  spec body exactly, including the prescribed *order* of the three rejections:
  no row -> `anyhow!("Game not found")`; `is_finished` -> `GameAlreadyFinished`;
  `updated_at != expected` -> `StaleStateConflict`. `SELECT ... FOR UPDATE` is
  present, so the lock is genuinely held to commit. `GameAlreadyFinished`
  (`:627-630`) is a distinguishable `thiserror` type as required, not a string.
  This is the load-bearing piece of the package and it is right.
- **Task 2.1 - `concede_game`** (`:320-339`). Takes `expected_updated_at`, claims
  first, *and* carries the belt-and-braces
  `AND updated_at = $2 AND is_finished = false` on the `UPDATE games`, with 0
  rows mapped to `StaleStateConflict`. The spec asked for both; both landed.
- **Task 2.3 - `undo_game`** (`:534-617`). All three prescribed additions are
  present and correct: the claim (`:544`); the in-transaction
  `undo_game_state IS NOT NULL` re-verify with `StaleStateConflict` on NULL
  (`:546-556`) - this is the addendum wd F15 asked for and it was implemented in
  full; and `AND updated_at = $4 AND is_finished = false` on the `UPDATE games`
  with 0 rows -> `StaleStateConflict` (`:558-569`). The spec's required verbatim
  rationale comment about not touching rating fields is present word-for-word
  (`:609-614`).
- **D-3 option A honoured.** The spec's "single most important line" was *no
  rating rewind of any kind*. There is none: `undo_game` touches no
  `rating_change`, `rating_before`, `game_type_users.rating` or `peak_rating`,
  and no delta-reversal or recompute code exists anywhere in `rating.rs`. The
  critical (wd F14) is closed by making the state unreachable, exactly as decided.
- **Task 5 - rating idempotency guard, correctly *narrowed*.** The spec said drop
  the `change == 0` skip in the second loop only and keep it in the first.
  `rating.rs:188` keeps `if change == 0 { continue; }` in the
  `game_type_users` loop; `rating.rs:205-216` writes `rating_change` and
  `rating_before` for every player in the map with no zero-skip. This is the one
  place in the unit where a subtle "which loop" instruction was followed
  precisely, and `apply_rating_changes_writes_zero_change` (`rating.rs:863-925`)
  asserts every clause of its spec row including the second-call no-op.
- **Task 4 - the email copies really were collapsed.** The spec's own structural
  assertion was that
  `rg "crate::db::undo_game|crate::db::concede_game" rust/web/src/email/commands.rs`
  must return nothing after Task 4. It returns nothing. `run_concede` and
  `run_undo` are pure delegations to the cores with no `is_finished`, `left_at`
  or `replacement_bot_available` logic of their own, and `run_restart` / `run_end`
  were not touched (the file has exactly two hunks). Defect 3 - the duplication
  that made two bugs into six - is genuinely closed, and this is the part of the
  package that most justified its existence.
- **Task 3 tail - `can_undo`.** `server_fns.rs:360-363` matches the spec's
  prescribed expression exactly, including the `!ge.game.is_finished` conjunct
  that brings it into line with `can_concede` / `can_end_game`.
- **Task 7 - docs.** `docs/CODING.md:653-674` is byte-identical to the spec's
  prescribed block, and the `docs/ARCHITECTURE.md` `is_finished`-is-one-way
  clause is verbatim. (What the rule leaves out is F-120; the text itself is
  faithful.)
- **Test roster.** All eight new spec-named tests are present, and all six
  pre-existing tests the spec required to survive are still there. The
  *names* and *error-type* assertions landed in full - F-114 is about the state
  assertions inside three of them, not about missing tests.
- **Task 3 - shared cores.** `ActingPlayer` (`server_fns.rs:827-830`),
  `undo_core` (`:846-915`), `concede_core` (`:918-981`) and
  `conflict_or_internal` (`:833-843`) all match the prescribed signatures,
  ordering and user-facing strings. The `#[server]` wrappers are reduced to
  context extraction, `get_current_user`, the core call, `broadcast_and_trigger`
  and `notify_game_emails` - the caller-owns-post-commit-side-effects shape the
  spec pointed at `restart_core` for.
- **The TOCTOU window `undo_core` actually has is guarded.** `undo_core` makes an
  HTTP round trip to the game service (`:884-893`) between the pool read and the
  write - the exact window Defect 2 described. `undo_game`'s claim closes it, and
  the HTTP call sits *outside* the transaction rather than inside a `FOR UPDATE`
  (contrast WP-79's finding in Unit 07). Correct on both counts.
- **`replacement_bot_available` and `pick_replacement_bot` use the same predicate**
  (`can_replace_humans = true AND enabled = true`, `bots.rs:81,103`), so
  `concede_core`'s availability probe cannot disagree with the later pick. This
  is the F-115-shaped mismatch that is *not* present.
- **No game crate involvement.** The breakdown's concern that a shared-core
  extraction might have left a game crate with its own finish-path bypass does
  not apply: `git show 9ba3736b -- rust/game/` is empty.

## Coverage gaps

Things this unit could not close, with who should pick them up.

- **F-119 vs WP-38's wedge-recovery sweep (Unit 05).** If that sweep re-derives
  status from the game service rather than gating on `is_turn = true`, it masks
  F-119 in production. It does not make the write correct, and it does not help
  the human whose opponent's turn never fires between sweeps, but it changes the
  severity. **Unit 05 owns WP-38; this needs a cross-check at unified-report
  time.** F-119's severity is stated on the assumption the sweep gates on
  `is_turn`.
- **`left_at` conflates two concepts** (eliminated-by-play vs left-the-game) and
  is written by four call sites with no single owner. F-116 and F-117 are both
  symptoms of that conflation, and F-113's `count_active_humans` reads it as a
  third thing again. A proper fix is a schema change, which is larger than any
  finding here. Recommend the remediation plan carry it as one item, not four.
- **No test in this package exercises a live game after a concede-with-replace.**
  Every concede test asserts the immediate row state (`left_at`, `game_bot_id`,
  place, rating) and stops. Nothing asserts that the game is still *playable*
  afterwards - which is why F-119 survived a package that shipped seven new
  guard tests. This is the shape of coverage gap the session should name: the
  tests assert the write, never the invariant the write exists to preserve.
- **Nothing in the repo tests `compute_ranked_placings` against a state any
  finish path actually produces.** Its three unit tests are hand-built
  `PlacingInput` vectors; no test runs `concede_game` or `end_game` and then
  reads `ranked_placing`. F-117 is the consequence.
- **The 83 generated sqlx cache files were not reviewed** and are out of scope
  for correctness. Checked and clean: `rust/.sqlx/` does **not** exist at HEAD
  (the commit's intermediate two-directory state did not survive), only
  `rust/web/.sqlx/` with 137 files, and `scripts/rust-ci-commands.sh:24` enforces
  `cargo sqlx prepare --check` scoped to `web`. No carry-forward.
- **`9ba3736b` bundles 224 lines of `docs/reviews/.../planning/` process state
  into a code commit** (`EXECUTION-STATE.md`, `WP-82-LEAD-STATE.md`). Not a
  defect; noted because it is the same mixed-commit pattern the breakdown warned
  about for `62b293df`, and it inflates the commit's apparent scope.

## Carry-forwards for other units

- **Unit 05:** cross-check WP-38's bot-turn wedge-recovery sweep against F-119
  (above). This is the one open dependency in this unit.
- **Unit 07 (owns export/import):** `rust/web/src/game/import.rs:109,124` is the
  **only** site in the tree that writes a non-NULL `undo_game_state` other than
  `update_game_command_success` - it inserts `player.undo_game_state` verbatim
  from an import bundle. Every other write forces NULL. That means an imported
  game can arrive with a caller-supplied undo snapshot that no game-service call
  ever produced, and `undo_game` will replay it into `games.game_state` after
  only checking that it is non-NULL (`game_write.rs:546-556`). Unit 07 should
  check what validates an import bundle's `game_state`/`undo_game_state`.
- **Unit 05:** as above, WP-38's wedge-recovery sweep vs F-119.
- **Not a carry-forward, recorded to prevent re-derivation:** the `game_bots`
  duplicate-name question is answered - `UNIQUE (game_id, name)` exists
  (`migrations/003:15`) and `pick_replacement_bot` has no `ON CONFLICT`. That is
  folded into F-111 rather than raised separately.

## Notes that are not findings

- **One pre-existing test assertion was reversed by `9ba3736b`** -
  `finishing_a_three_player_game_rates_all_pairs` changed
  `assert_eq!(c_p1, None)` to `assert_eq!(c_p1, Some(0))` (`rating.rs:484`).
  Checked against pattern 4b and it is **not** an instance: Task 5's whole point
  was to start writing `rating_change = 0` so the idempotency guard arms on an
  exact tie, so the old assertion had to change and the new one asserts the
  intended new behaviour. The test was not on the spec's six-test "must pass
  unmodified" list. The accompanying comment cites WP-40 Task 5 by name. Clean.
- The other 19 deleted lines across `game_write.rs` and `rating.rs` are call
  sites gaining `expected_updated_at` plus one comment. No other assertion moved.
