# WP-40 - undo/concede TOCTOU + ratings integrity

**Goal:** make every state-mutating game action outside the move path carry the
move path's optimistic-locking discipline, and forbid undo across the
finished/rated boundary it cannot revert - so the rating-corruption critical
becomes unreachable rather than compensated for.

**Scope (8 findings):** wd F14 (critical), wd F15, wd F16, wfe F19, wfe F20,
wfe F22, ws F34, ws F38. Severity 1c/6M/1m.

**Files touched:** `rust/web/src/db.rs`,
`rust/web/src/game/server_fns.rs`, `rust/web/src/email/commands.rs`.

## Binding decisions (already answered - do not re-litigate)

**D-3 = option A** (`planning/decisions-needed.md`, ANSWERED 2026-07-25):
> Forbid undo once a game is finished, making the ratings corruption
> **unreachable**. Do **NOT** attempt any rating rewind - no delta-reversal
> code, no recompute. The missing rating rewind is therefore explicitly out of
> scope for WP-40.

**D-4 = option A** (same file, ANSWERED 2026-07-25):
> Share `undo_core` / `concede_core` between the web and email paths "so the
> missing concurrency guards are fixed once", with the `is_finished` /
> `updated_at` guards living in `db.rs`.

**SUPERSEDED NOTE - read this before `work-packages.md`.**
`planning/work-packages.md`'s WP-40 entry ends with *"rewind via stored
deltas"*. That note **predates D-3 and is void.** Do not implement a rewind.
Likewise ws F34's own recommendation offers *"recompute on next finish"*; that
was already established to **double-count** ratings (it re-rates against
`game_type_users` values that already absorbed the voided deltas), and under
option A it is doubly moot because a rated game can no longer be undone.
**Also: WP-82 (`db.rs` module split) is now a HARD PREDECESSOR of WP-40 per
`landing-order.md` 7.1 - this spec's older "the split lands after your db.rs
edits" wording is WRONG and following it would do harm.**

## How to use this spec

Every code reference below is by **file path + function/type name**. Line
numbers appear only as navigational hints and are marked *approximate, verify*.
**Locate and read each named function before editing it. If what you find does
not match this spec's description of it, STOP and report** - do not improvise a
merge. This matters more than usual here: the review snapshot is ~500 lines
behind live `db.rs` (the #47 concede/end-game work), and two predecessor
packages edit the same three functions.

## Predecessors and overlaps

- **WP-41 (`planning/specs/WP-41-db-quality-pass.md`) MUST land first.** It
  touches all three of your db.rs functions, deletion-only:
  - `concede_game`, `concede_game_replace`, `end_game`, `undo_game`,
    `apply_rating_changes`: the manual `updated_at = NOW()` clauses are
    **removed** (trigger-maintained; WP-41 Task 1). Expect the `SET` lists to
    be one clause shorter than what you read today.
  - `apply_rating_changes`: the all-pairs loop header is rewritten to slice
    form (WP-41 Task 5). Its two `if change == 0 { continue; }` write loops are
    explicitly left alone for you (WP-41 states this).
  - `update_game_command_success`: `is_finished` becomes **sticky**
    (`is_finished = ($2 OR is_finished)`, WP-41 Task 3), leaving `undo_game` the
    only un-finish path in the codebase. Your `is_finished` guard makes that
    path unreachable-on-finished; keep `undo_game`'s `finished_at = NULL` /
    `is_finished = $2` writes as they are (they become no-ops, not bugs).
  - WP-41 adds a db.rs module doc header enumerating sections; if you add
    functions, keep it accurate.
  - **If WP-41 has NOT landed:** stop and say so. Do not pre-apply its
    deletions and do not restructure around them; rebasing two deleted clauses
    onto your restructure is trivial, the reverse is not (WP-41's own stated
    reason for the order).
- **WP-59 (`planning/specs/WP-59-inbound-processing-quality.md`) MUST land
  first** for two things you consume in `email/commands.rs`:
  - `classify_server_fn_error(context, ServerFnError) -> CommandError` (WP-59
    Task 9), which maps a redacted internal (`crate::error::INTERNAL_ERROR_MESSAGE`)
    to `CommandError::Internal` and everything else to `CommandError::User`.
    This is how your `*_core` errors must reach the email reply.
  - The one-line `run_restart` `map_err` fix at the `restart_core` call site.
    WP-59 owns that line; you must not touch `run_restart`.
- **WP-54 (`planning/specs/WP-54-frontend-ux-error-handling.md`) Task 1** adds
  the shared error slot that finally *renders* `undo_action` / `concede_action`
  failures in `components/game.rs`. Either order. **WP-40 does not edit
  `components/game.rs`** - your guard messages are returned as
  `ServerFnError`; WP-54 makes them visible.
- **File-level collisions (no ordering constraint, disjoint regions):** WP-45
  (bot-slot validation) and WP-47 (`game_visibility` gates) also edit `db.rs`
  and `game/server_fns.rs`. The **db.rs module split (ws F42)** is now
  **WP-82** and is a **HARD PREDECESSOR: WP-82 -> WP-40.** It lands **first**;
  rebase this package onto the post-split `rust/web/src/db/` tree, where each
  db.rs edit below lands in the relevant `db/*.rs` submodule. See
  `landing-order.md` 7.1.
- **Routed in from WP-41:** `concede_game`'s 2-player assumption is a
  `debug_assert!` only (hint ~db.rs:1315, *approximate, verify*), so a release
  build silently gives place 1 to every non-conceder in a 3+-player game and
  `apply_rating_changes` then rates that. WP-41 routed this here because the
  correct behaviour depends on D-3/the concede restructure. Task 6.

## 1. Root cause

*Symptom:* undo on a just-finished game permanently corrupts ratings, and
undo/concede can silently overwrite a concurrent move or a real result.

*Defect 1 - a state-reverting operation with no knowledge of its derived
projections.* `db::undo_game` reverts the authoritative state
(`games.game_state`, `is_finished`, `finished_at`, per-player `place`,
`is_turn`, `undo_game_state`) but `apply_rating_changes` derives a **second,
non-revertible projection** from the same transition: `game_players.rating_change`
/ `rating_before` plus accumulated `game_type_users.rating` / `peak_rating`.
`rating_change` doubles as `apply_rating_changes`' idempotency token, so
reverting the state while leaving the projection behind is worse than either
alone: the voided ELO sticks *and* the guard suppresses rating of the real
outcome forever. Nothing in the type system or the schema ties the two together,
so the revert path was written without noticing the projection existed.

*Defect 2 - guard checks living outside the transaction that needs them.*
`update_game_command_success` established the correct discipline: the caller
passes `ge.game.updated_at`, the games UPDATE carries `AND updated_at = $n`,
and 0 rows affected becomes the distinguishable `db::StaleStateConflict`
(consumed in `game/mod.rs::execute_command` as
`ExecuteCommandError::Conflict`). `undo_game`, `concede_game` and
`concede_game_replace` were written later/elsewhere and check nothing: the
server fns and the email commands test `ge.game.is_finished` against a **pool
snapshot** read before an HTTP round-trip to the game service, then write
unconditionally. A check against a snapshot is not a guard; it only narrows the
window.

*Defect 3 - the duplication that made 2 into 6.* `email/commands.rs`'s
`run_concede` / `run_undo` are near line-for-line copies of `game/server_fns.rs`'s
`concede_game` / `undo_game` (same snapshot read, same checks, same service
round-trip, same broadcast/notify tail), so the same two defects are present at
four entry points. `restart_core` in `game/server_fns.rs` is the in-repo proof
that this feature already knows the right shape - shared core, caller owns
post-commit broadcast/notify, serialize on the game row with `FOR UPDATE`.

## 2. Complete solution

### Task 1 - one claim helper in `db.rs`, next to `StaleStateConflict`

Add, beside the existing `StaleStateConflict` (hint ~db.rs:1857, *approximate,
verify*):

```rust
/// Distinguishable so callers can tell "you cannot do this any more" apart
/// from "someone else moved first"; the two need different user-facing text.
#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
#[error("Game is already finished")]
pub struct GameAlreadyFinished;
```

Add a private helper used by every mutating game-lifecycle write in Task 2:

```rust
/// Claims the game row for a state-mutating write. Takes the row lock
/// (`FOR UPDATE`) so a concurrent `update_game_command_success` blocks and
/// then fails its own `updated_at` guard, then enforces (a) the game is not
/// finished and (b) it has not changed since the caller's snapshot. Mirrors
/// the optimistic-locking discipline of `update_game_command_success` and the
/// `FOR UPDATE` serialization of `game::server_fns::restart_core`.
#[cfg(feature = "ssr")]
async fn claim_unfinished_game_tx(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
    expected_updated_at: time::PrimitiveDateTime,
) -> Result<()>
```

Body: `SELECT is_finished, updated_at FROM games WHERE id = $1 FOR UPDATE`,
then in order - no row → `anyhow!("Game not found")`; `is_finished` →
`GameAlreadyFinished`; `updated_at != expected_updated_at` →
`StaleStateConflict`; else `Ok(())`. The lock is held to commit, so every write
after the claim is serialized against the move path.

### Task 2 - guard the three lifecycle writers

All three gain a final `expected_updated_at: time::PrimitiveDateTime`
parameter and call `claim_unfinished_game_tx` as the first statement after
`pool.begin()`:

- `db::concede_game` (hint ~1291) - closes wd F16, wfe F19. Belt-and-braces:
  also append `AND updated_at = $2 AND is_finished = false` to its
  `UPDATE games SET is_finished = true, ...` and treat 0 rows as
  `StaleStateConflict`. (This is the exact WHERE-clause shape
  `update_game_command_success` uses; the claim makes it unreachable, keep it
  so the invariant is local to the statement.)
- `db::concede_game_replace` (hint ~1382) - **in scope**: it is the branch the
  server fn/email command take whenever a replacement bot exists, it is
  reachable from the same unguarded call sites, and it writes `game_players`
  (bot swap, `left_at`, `undo_game_state = NULL`) plus a log **without touching
  the `games` row at all** - so it has no UPDATE to hang a WHERE clause on and
  the claim helper is the only available guard. Without it, half the concede
  traffic stays racy and wfe F19 is only half closed.
- `db::undo_game` (hint ~1537) - closes wd F15, wfe F20, and by making
  `is_finished` a hard precondition closes **wd F14 / ws F34** with no rating
  work. Two additions beyond the claim:
  1. append `AND updated_at = $n AND is_finished = false` to its
     `UPDATE games SET game_state = ...` and map 0 rows to
     `StaleStateConflict`;
  2. take the acting `game_player_id` and re-verify inside the transaction that
     its `undo_game_state IS NOT NULL` (`SELECT undo_game_state FROM
     game_players WHERE id = $n AND game_id = $1`); NULL → `StaleStateConflict`.
     Without this a doubled undo request replays a stale snapshot even though
     the row lock was clean.

Do **not** add, clear or recompute any rating field in any of them (D-3 A).
Add a comment in `undo_game` saying exactly that and why - it is the answer ws
F34 asked for in its "if they cannot" branch:

```rust
// Rating fields (`game_players.rating_change`/`rating_before`,
// `game_type_users.rating`) are deliberately NOT touched here: a finished
// game can no longer be undone (see the claim above), so no rated game ever
// reaches this function. Rewinding ratings is out of scope by decision
// (review 2026-07-23, D-3 option A); if undo-after-finish is ever allowed
// again, the rewind must land in the SAME transaction as this revert.
```

### Task 3 - `undo_core` / `concede_core` in `game/server_fns.rs` (wfe F22)

Model them on `restart_core` (hint ~986): `pub(crate)`, `#[cfg(feature = "ssr")]`,
`-> Result<_, ServerFnError>`, doc comment stating the guarantee, **caller owns
post-commit broadcast/notify**.

```rust
/// Which player is acting. The web path knows a `user_id`, the email path a
/// `game_player_id`; resolving inside the core means the game is fetched once.
#[cfg(feature = "ssr")]
pub(crate) enum ActingPlayer { User(Uuid), GamePlayer(Uuid) }

pub(crate) async fn undo_core(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: Uuid,
    actor: ActingPlayer,
) -> Result<crate::db::GameExtended, ServerFnError>;

pub(crate) async fn concede_core(
    pool: &sqlx::PgPool,
    game_id: Uuid,
    actor: ActingPlayer,
) -> Result<crate::db::GameExtended, ServerFnError>;
```

The returned `GameExtended` is the `before` snapshot both callers already pass
to `notify_game_emails`. Move into the cores, verbatim from the existing
`game/server_fns.rs::undo_game` / `concede_game` bodies (they are the more
complete copies):

- `undo_core`: `find_game_extended` → resolve actor (`"You are not a player in
  this game"`) → **new** `if ge.game.is_finished { return
  Err(ServerFnError::new("This game is finished and can no longer be
  undone.")) }` (fast, friendly rejection; the db claim is the authority) →
  `undo_game_state` or `"No undo state available"` → `client::request` with
  `Request::Status` → `status_fields` → `db::undo_game(..., player.game_player.id,
  ge.game.updated_at)`.
- `concede_core`: `find_game_extended` → `is_finished` early-out (keep the
  existing `"Game is already finished"` text) → resolve actor → `left_at`
  check → active-human count (`count_active_humans`) →
  `db::replacement_bot_available` → the existing three-way branch
  (`concede_game_replace` / `concede_game` / the
  `"Concede is not available: no replacement bot configured"` refusal), each db
  call now passing `ge.game.updated_at`.

Both cores map db errors through one shared private helper so the two conflict
types get user-facing text instead of `internal`'s redaction:

```rust
fn conflict_or_internal(context: &'static str, e: anyhow::Error) -> ServerFnError {
    if e.downcast_ref::<crate::db::GameAlreadyFinished>().is_some() {
        return ServerFnError::new("Game is already finished");
    }
    if e.downcast_ref::<crate::db::StaleStateConflict>().is_some() {
        return ServerFnError::new(
            "The game changed while this was being processed; nothing was changed. Please try again.",
        );
    }
    internal(context)(e)
}
```

(`internal` is `crate::error::internal`, already imported in this module; the
downcast idiom is the one `game/mod.rs::execute_command` uses for
`StaleStateConflict`.)

Then reduce the two `#[server]` fns to: context extraction, `get_current_user`,
`let before = core(...).await?;`, `broadcast_and_trigger`, `notify_game_emails`.
Also in `get_game_details`, change `can_undo` to
`!ge.game.is_finished && player.and_then(|p| p.game_player.undo_game_state.as_ref()).is_some()`,
matching the `!ge.game.is_finished &&` shape `can_concede` / `can_end_game`
already use - the UI must not offer an action the server now always rejects.

### Task 4 - collapse the email copies (wfe F19, F20, F22)

In `email/commands.rs`, `run_concede` and `run_undo` become, in full:

```rust
let before = concede_core(ctx.pool, ctx.game_id, ActingPlayer::GamePlayer(ctx.game_player_id))
    .await
    .map_err(|e| classify_server_fn_error("concede", e))?;
crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, ctx.game_id).await;
crate::email::notify::notify_game_emails(ctx.resend, ctx.pool, ctx.http_client, ctx.game_id, Some(before)).await;
Ok(CommandReply::Status("You conceded.".to_string()))
```

and the `undo_core` equivalent (`http_client` argument, `"Undo applied."`).
Both keep their existing reply strings. `classify_server_fn_error` (WP-59) sends
every message the cores produce back as a `CommandError::User` reply and only
the redacted-internal case to `CommandError::Internal`, so a conceded race or a
finished-game undo now gets an explanatory email instead of a silent clobber -
identical text to the web path, because it is the same string constant path.
Leave `run_end` alone (`db::end_game` is not in this package's finding set).

### Task 5 - arm the rating idempotency guard (ws F38)

In `db::apply_rating_changes`, both write loops skip players whose computed
change is 0 (`if change == 0 { continue; }` in the `for p in &rated_players`
loop and in the `for p in &players` loop). Delete the skip in the **second**
loop only, so `game_players.rating_change = 0` and `rating_before` are always
written for every rated player and the guard `players.iter().any(|p|
p.rating_change.is_some())` is armed even for an exact tie between equally
rated players. Keep the skip in the first loop: a zero-delta
`UPDATE game_type_users SET rating = rating + 0` is pure noise (and would bump
`updated_at` via the trigger). This makes "finished-and-rated ⇒ rating_change
set" a true invariant, which is what the finished-game undo ban now relies on
being reliable.

### Task 6 - `concede_game`'s 2-player assumption (routed in from WP-41)

Replace the `debug_assert!(players.len() == 2, ...)` with a real error inside
the transaction: if `players.len() != 2`, return
`Err(anyhow!("concede_game requires exactly 2 players, found {}", players.len()))`.
Both callers already gate on `active_humans == 2` before choosing this branch,
so this is unreachable in practice - which is the point: a release build must
not silently mis-place and then rate a 3+-player game.

## 3. Constraint going forward (the reviewable rule)

Two mechanically checkable rules, to be stated in `docs/CODING.md` (Task 7):

1. **Every `db.rs` function that reads game state and then writes it must take
   an `expected_updated_at` and claim the row.** Concretely: if a function's
   body contains an `UPDATE games` or an `UPDATE game_players` for a game whose
   identity came from a caller's earlier snapshot, its signature must end in
   `expected_updated_at: time::PrimitiveDateTime` and its first statement after
   `begin()` must be `claim_unfinished_game_tx`. A reviewer greps for
   `UPDATE game` in db.rs and checks each hit's enclosing function for the
   parameter. Conflicts must be returned as the distinguishable
   `StaleStateConflict` / `GameAlreadyFinished` types, never as a bare
   `anyhow!` string, so callers can produce correct user-facing text.
2. **A state-reverting operation is forbidden across any boundary it cannot
   revert.** Reverting `games.game_state` is cheap; reverting
   `game_type_users.rating` is not, and `game_players.rating_change` is
   load-bearing as an idempotency token. So the revert path refuses to cross
   the finished/rated boundary rather than compensating for it. Any future
   operation that reverts state must enumerate the derived projections of the
   transition it undoes and either revert them in the same transaction or
   refuse.

## 4. Documentation updates

`docs/CODING.md` already has a **Database** section that opens with the
`games.updated_at` trigger note - the natural home. Append after that note:

```markdown
**Read-then-write on a game row needs an `updated_at` guard, not a snapshot
check.** `update_game_command_success` is the reference shape: the caller
passes `ge.game.updated_at`, the write carries `AND updated_at = $n`, and 0
rows affected becomes `db::StaleStateConflict` - a distinguishable type, so
callers can say "someone moved first" instead of "internal error". Every
mutating game-lifecycle function (`concede_game`, `concede_game_replace`,
`undo_game`) takes `expected_updated_at` and opens its transaction with
`claim_unfinished_game_tx`, which locks the row `FOR UPDATE` and rejects both a
finished game (`db::GameAlreadyFinished`) and a stale snapshot. Checking
`ge.game.is_finished` in a server fn is a courtesy for the error message only;
it is never the guard, because the game service round-trip sits between the
read and the write.

**A finished game cannot be undone.** `undo_game` refuses once
`games.is_finished` is true. Finishing a game runs `apply_rating_changes`,
which mutates `game_type_users.rating`/`peak_rating` and stamps
`game_players.rating_change` as its own idempotency token; `undo_game` reverts
game state only, so undoing a finish would leave the voided ELO applied *and*
suppress rating of the real outcome forever. Rewinding ratings is deliberately
not implemented (review 2026-07-23, decision D-3 option A). If undo-after-finish
is ever wanted, the rewind must land in the same transaction as the state
revert.
```

`docs/ARCHITECTURE.md` "Database Schema" describes `games` as *"Active and
finished game instances"* and `game_players` as *"player position and
player-specific state"* with no lifecycle semantics; extend the `games` bullet
with one clause: *"`is_finished` is one-way for a rated game - a finished game
can be deleted by an admin but never un-finished (see docs/CODING.md,
Database)."* No other doc describes undo semantics (`docs/CODING.md`'s only
current mention of undo is the Playwright-scope sentence in Testing
Conventions; `docs/ARCHITECTURE.md`'s is the `can_undo` field in the game
interface contract). Do not invent a new lifecycle doc.

## 5. Regression-test plan

Tests live in `rust/web/src/db.rs`'s inline `mod tests` (this is where the
existing coverage is) using `#[sqlx::test]` with the local fixtures
`make_user`, `make_game_type_and_version`, `make_game_with_players`,
`find_rating_change`, `game_type_rating`. Real neighbours to copy the shape
from: `undo_game_restores_state_and_clears_undo`,
`concede_game_marks_finished`, `concede_game_assigns_places_and_rates`,
`concede_game_replace_swaps_in_bot`, `ratings_use_ranked_placing_and_skip_pure_bots`.
`docs/CODING.md` Testing Conventions makes tests **mandatory** for db.rs
changes.

| Case | Setup | Expected |
| --- | --- | --- |
| `undo_game_rejects_finished_game` | 2 humans; `update_game_command_success` with `status.is_finished = true`, placings `[0,1]`, `can_undo = true` (so the stash survives) | `undo_game(...)` → `Err` downcasting to `GameAlreadyFinished`; `game_state`, `is_finished`, `finished_at`, both `place` and both `rating_change` **unchanged**; the `"used an undo"` log absent |
| `undo_game_rejects_stale_updated_at` | stash undo state for p0, then land a second `update_game_command_success` (p1's move) | `undo_game` with the **pre-move** `updated_at` → `Err` downcasting to `StaleStateConflict`; `games.game_state` still p1's post-move state; **both** players' `undo_game_state` still exactly as the second move left them (this is the "destroys nothing" assertion wd F15 is about) |
| `undo_game_rejects_consumed_undo_state` | stash, undo once successfully, re-issue `undo_game` with the refreshed `updated_at` | `Err` → `StaleStateConflict`; single `"used an undo"` log |
| `concede_game_rejects_finished_game` | finish via `update_game_command_success` with placings `[0,1]`, then concede | `Err` → `GameAlreadyFinished`; places still `1`/`2` from the real finish, `rating_change` values unchanged - no places/ratings disagreement |
| `concede_game_rejects_stale_updated_at` | snapshot `updated_at`, land a move, then concede with the stale value | `Err` → `StaleStateConflict`; `is_finished` false, no places written |
| `concede_game_replace_rejects_finished_game` | seed a `can_replace_humans` bot (see `concede_game_replace_swaps_in_bot`), finish the game, then call | `Err` → `GameAlreadyFinished`; no `game_bots` row added, `left_at` still NULL |
| `concede_game_requires_two_players` | 3 humans | `Err` (Task 6), no places written |
| `apply_rating_changes_writes_zero_change` | 2 equally-rated humans, exact tie (`ranked_placing`/`place` equal for both) | both `rating_change == Some(0)`, `rating_before == Some(1200)`, `game_type_users.rating` still 1200; a **second** finish attempt on the same game is a no-op (guard armed) |
| Existing tests must pass **unmodified** | `undo_game_restores_state_and_clears_undo`, `concede_game_marks_finished`, `concede_game_assigns_places_and_rates`, `concede_game_replace_swaps_in_bot`, `finishing_a_game_captures_rating_before`, `update_game_command_success_keeps_first_undo_stash_in_a_run` | only their call sites gain the new `expected_updated_at` argument (`ge.game.updated_at`) - no assertion changes |

**Path parity (wfe F22 is the mechanism, not a test):** the email and web paths
are identical *because they call the same core*. Assert that structurally
rather than duplicating DB tests: `grep -n "crate::db::undo_game\|crate::db::concede_game"
rust/web/src/email/commands.rs` must return **nothing** after Task 4, and
`run_concede`/`run_undo` must contain no `is_finished`, `left_at` or
`replacement_bot_available` logic of their own. Keep the existing
`email/commands.rs` `mod tests` unit tests green.

**Commands for the implementer to run** (this spec's author ran none of them;
run from `/home/beefsack/Development/brdgme/rust`, always single-package per
`AGENTS.md`):

- `cargo test -p web --features ssr undo_game`
- `cargo test -p web --features ssr concede`
- `cargo test -p web --features ssr rating`
- `cargo clippy -p web --all-targets --features ssr -- -D warnings`
- `cargo fmt --all -- --check`
- **`cargo sqlx prepare` is required:** you change SQL text inside
  `sqlx::query!` macros (`concede_game`, `undo_game`, `apply_rating_changes`)
  and add a new macro query in `claim_unfinished_game_tx`, so `.sqlx/` goes
  stale and `cargo sqlx prepare --check` fails until regenerated. Use the
  scratch-database flow WP-41's Global Constraints documents, then
  `(cd web && cargo sqlx prepare -- --tests --features ssr --all-targets)`.
- Full gate before commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh`.

## 6. Non-goals (do not scope-creep)

- **NO rating rewind or recompute of any kind** - no delta reversal, no
  `game_type_users` adjustment, no clearing `rating_change`/`rating_before`
  anywhere. D-3 option A. This is the single most important line in the spec.
- **NO grace window** for undo-after-finish (D-3 option C, rejected).
- **NO change to how ratings are computed**: `elo_*` helpers,
  `write_ranked_placings`, the all-pairs loop and the `ranked_placing.or(place)`
  precedence are untouched. Task 5 changes *where a zero is recorded*, nothing
  arithmetic.
- **NO db.rs module split** (ws F42) - that is **WP-82**, a separate package,
  and it lands **before** this one (`WP-82 -> WP-40`, `landing-order.md` 7.1).
  Do not perform or extend the split here; just target the post-split
  `db/*.rs` submodule that now holds each named function.
- **NO unrelated WP-41 cleanups**: do not sweep `updated_at = NOW()`, do not
  touch `send_friend_request`, `choose_colors`, `is_user_admin`,
  `build_game_type_user`, or add WP-41's coverage tests.
- **NO broadening to other mutating db.rs functions.** `end_game`,
  `delete_game`, `mark_game_read`, `insert_game_logs_tx`,
  `update_game_command_success` and the proposal writers keep their current
  signatures. The three guarded functions are exactly the ones the eight
  findings name (plus `concede_game_replace`, which is the same call site's
  other branch and has no guard at all).
- **NO frontend work**: `components/game.rs` is WP-54's (error slot),
  `run_restart` is WP-59's, typed error enums for `restart_core` are explicitly
  rejected by WP-59.
- **NO new email verbs, reply-text redesign, or `run_end` changes.**

## 7. Finding-recommendation audit

| Finding | Original recommendation | Verdict |
| --- | --- | --- |
| **wd F14** (critical) | "Reject undo when `ge.game.is_finished` ... plus a matching guard inside `db::undo_game`" | **RIGHT, and it is what we implement.** It offered "or make `db::undo_game` rewind rating changes atomically" as an alternative; D-3 A rejects that alternative. The "matching guard inside db.rs" half is the essential part - the server-fn check alone is only a nicer error message. Do not revert to a server-fn-only check. |
| **wd F15** | Pass `ge.game.updated_at` in, add `AND updated_at = $n`, return `StaleStateConflict` on 0 rows, "additionally verify the player's `undo_game_state` is still non-NULL inside the transaction" | **RIGHT in full, including the `undo_game_state` addendum** (Task 2.3). Implemented as `claim_unfinished_game_tx` + the WHERE clause; the claim's `FOR UPDATE` is a strengthening, not a substitution. |
| **wd F16** | Lock the game row `FOR UPDATE` and bail if finished, **or** `WHERE id = $1 AND NOT is_finished` | **RIGHT; we do both** (the claim gives a distinguishable error and covers `concede_game_replace`, which has no games UPDATE to hang a WHERE clause on; the WHERE clause keeps the invariant local to the statement). Its observation that `restart_core` already does this correctly is the pattern we copied. |
| **wfe F19** | `WHERE id = $1 AND is_finished = false`, map 0 rows to a "game already finished" conflict surfaced to the user | **RIGHT.** Incomplete only in that it predates `concede_game_replace` (#47), which is now the default concede branch and equally unguarded. |
| **wfe F20** | Add the `updated_at` guard mirroring the move path; "reject undo on finished games **or** clear `rating_change` inside the undo transaction" | **RIGHT on the guard; the second half of the disjunction is superseded** - clearing `rating_change` alone would still leave `game_type_users.rating` carrying the voided delta. D-3 A takes the "reject" branch. |
| **wfe F22** | Extract `concede_core`/`undo_core` (pool + resolved game_player) shared by server fns and email, "as done for restart" | **RIGHT, and it is the vehicle for the whole package.** Refined only in the actor argument: an `ActingPlayer` enum instead of a pre-resolved player, so the game is fetched once and the web path needs no extra query. |
| **ws F34** | "clear `rating_change`/`rating_before` in `undo_game` and either rewind `game_type_users` by the stored deltas **or recompute on next finish**" | **Diagnosis RIGHT (verification: CONFIRMED (strengthened) - reachability is stronger than the finding's own UNCERTAIN framing, since the undo server path has no `is_finished` guard). Recommendation KNOWN-UNSOUND:** "recompute on next finish" **double-counts** - `game_type_users.rating` already absorbed the voided deltas, so recomputing from it applies them twice. The verification note flags exactly this. The delta-rewind variant is sound but rejected by D-3 A. We take the finding's own fallback instead - it asked for "a comment in `undo_game` stating why rating fields are left alone", and Task 2 writes that comment. |
| **ws F38** (minor) | "Write `rating_change = 0` (and `rating_before`) even when the change is 0, so the guard is reliable" | **RIGHT.** Narrowed in one way: only the `game_players` write loop drops the skip; the `game_type_users` loop keeps it, because `rating = rating + 0` is a pointless write that also bumps a trigger-maintained `updated_at`. Placed here (not WP-41) because the finished-game undo ban relies on the guard being reliable. |
