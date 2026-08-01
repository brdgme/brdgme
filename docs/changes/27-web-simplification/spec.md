# 27: rust/web simplification (skinny queries, dedup, client plumbing) - Design

> Extracted 2026-07-08 from `docs/plan/27-web-simplification.md` (superpowers layout
> migration). Content dates from 2026-07-07; this is a point-in-time decision
> record, not a living document.

**Status:** Paused 2026-07-07. WP4, WP1, WP2 (incl. 2a pinning tests), WP5
done, gated and committed (bc72a1f, cada5d7, c019c3a, 8ab1c33, 2fdd2a7).
Remaining work deferred - see "Deferred work" below.

**Goal:** Continue the Phase 17 skinny-payload direction inside `rust/web`:
replace fat `GameExtended` fetches with purpose-built queries where callers
use a sliver of the data, delete duplicated mechanisms (dual WS signals,
cloned log components, inlined broadcast epilogue, create/restart epilogue),
and strip mechanical boilerplate. Net effect: fewer lines, fewer queries,
one mechanism per job.

**Origin:** review session 2026-07-05 (leptos branch). Eleven findings were
risk-assessed; ten accepted, one rejected (`ANY($1)` batch user lookup -
silently-fewer-rows semantics buys nothing at current scale; do not do it).

## Explicitly out of scope

- Batch `ANY($1)` user lookup in `create_game_with_users_tx` (finding 11) -
  rejected in risk review: silent fewer-rows semantics needs a count check
  to be safe and buys nothing at current scale.
- Refactoring `find_game_extended` itself / its remaining fat callers
  (`get_game_details`, `undo_game`, `concede_game`, `restart_game` genuinely
  use the breadth). The `GameExtended` `Serialize` derive +
  `login_confirmation` leak concern: nothing serializes it today; if a WP
  finds the derive unused after its changes, drop `Serialize`/`Deserialize`
  from `GameExtended`/`GamePlayerExtended` as a freebie, otherwise leave.
- Per-player UPDATE loops in `update_game_command_success`/`undo_game`
  (unnest rewrites) - more complexity for no maintainability gain.
