# Dual Result Model

## Intent

Separate authoritative game placings from human competitive placings so games
with bots, replacements, departures, concessions, and early stops produce
correct durable results, ratings, stats, and presentation.

## Approved Semantics

- `game_players.place` is the authoritative bot-inclusive game placing supplied
  by game-service final placings, except the existing two-human platform
  concession forfeit.
- `game_players.ranked_placing` is the human-only competitive placing for ELO
  and competitive presentation/stats. Raw `points()` is never a ranking source.
- Voluntary last-human stop has no game places and a competitive result only
  when at least two humans participated.
- At normal finish, active humans use authoritative game-place order after bot
  removal with standard-competition ties. Departed humans follow in descending
  departure-event sequence, with ties within an event. Replacement-bot
  performance never improves a departed human.
- Two-player concession retains authoritative and competitive 1/2 forfeit.
- Finished notifications use authoritative `game_players.place` for normal
  game-service and two-human-concession finishes. A `last_human_stop` email
  header is exactly `Game ended early. No game result.` and does not present a
  winner, placing, points-derived order, or rating deltas as a game result.
- Replacement concession remains notification-free; no replacement-concession
  notification type is introduced.
- Pure bots have no competitive placing or ELO. Replaced humans remain human
  participants. One/zero-human games have no competitive result or ELO.
- One active human may stop early. With zero active humans, only humans in the
  latest departure event may stop, and all tied humans are authorized.
- Concede is available only while at least two active humans remain; at exactly
  one active human, End replaces Concede and terminates without bot replacement.
- Durable fields are nullable game end reason, player departure reason, and a
  deterministic positive departure-event sequence. Reasons are checked text:
  game-service finish, concession forfeit, last-human stop; conceded,
  timeout-replaced, eliminated, unknown legacy. Reasons are informational, not
  ranking inputs.
- One lifecycle event assigns a shared sequence to all simultaneous departures
  under game-row transaction serialization.
- Unfinished legacy departed rows initialize sequence from per-game `left_at`
  dense ordering, tie equal timestamps, and use `unknown legacy`; completed
  historical results and ratings remain unchanged. New code normalizes old-pod
  writes lacking metadata during rollout.
- Competitive profile/default stats, form, head-to-head, histograms, rating
  history, ELO, and recent markers use only `ranked_placing`; missing competitive
  placements yield no recent marker.
- The dual-result selector is limited to the main `/players/:name` profile for
  this change. The legacy `?bots=1` query is removed and ignored. The existing
  history route remains unchanged; extending selector/dual-result presentation
  to game-type and history pages is unscheduled backlog item #59.
- The separate Game results view uses authoritative `place`, includes bot seats,
  shows no result for early stop, and labels the value `Game placing`.
- Export/import schema v2 preserves result/departure/end metadata,
  replacement identity, and every player timestamp exactly: `created_at`,
  `updated_at`, `is_turn_at`, `last_turn_at`, and nullable `left_at`.
  `is_human` distinguishes pure bots from replacement-human seats, which retain
  both their human identity and `bot_name`. Exact-version rejection remains
  explicit; missing v2 fields have no defaults or compatibility path.

## Scope

- One additive migration, Rust web models, projections, writers, ranking,
  ratings, lifecycle actions, export/import, queries, UI, email, and tests.
- SSR/hydration structure remains stable: selected view is an unconditional
  input to the existing blocking resource; layout stays outside `Suspense`; no
  conditional or client-only resource boundary.

## Non-Goals

- Game-service protocol, gameplay/rules, points, timers/timeouts, retrospective
  repair, deployment manifests, and edits to existing migrations.

## Acceptance Criteria

1. Fresh migration enforces nullable checked schema.
2. Unfinished legacy departures initialize deterministically; completed history remains unchanged.
3. Normal finish stores only service places and ties, with no points fallback.
4. Two-human concession stores authoritative/competitive 1/2 and ELO.
5. Last-human stop has no game place and a competitive result only for 2+ humans.
6. Pure tests cover active/departed ordering, reverse events, event ties, bot removal, and active game-place ties.
7. Pure bots are unranked/unrated; replaced humans are ranked/rated.
8. Rating uses competitive places only, eligible human pairs only, once.
9. Competitive queries/markers use ranked placement; Game results use game placement and bot seats.
10. Completed historical rows stay untouched; no missing historical competitive places are invented.
11. Export/import v2 round-trips replacement identity and all result metadata; v1 rejection is explicit.
12. Web/email lifecycle authorization, reason, sequence, notifications, and stale-state behavior align.
13. SSR views preserve hydration structure and pass available evidence.

## Approval

- User approved this specification and the initial `spec.md`/`plan.md` artifacts.
- User and Orchestrator approved the 2026-08-05 clarification: Concede is
  available only while at least two active humans remain; at exactly one active
  human, End replaces Concede and terminates without bot replacement.
- User and Orchestrator approved the 2026-08-05 DRM-03c2 notification decision:
  replacement concession sends no notification; `last_human_stop` uses the
  exact no-result header; all other Finished notifications retain authoritative
  game-place presentation.
- User and Orchestrator approved the 2026-08-05 DRM-04 split into DRM-04a,
  DRM-04b, and DRM-04c. DRM-04a adds only the required export shape and
   compile-required import fixture literals while preserving schema version 1;
   DRM-04b writes the already-required fields on import; DRM-04c atomically
   flips to version 2 and explicitly rejects v1 envelopes. Player timestamps
   are preserved exactly.
- User and Orchestrator approved the 2026-08-05 DRM-05 clarification and serial
  DRM-05a..DRM-05e proposal: competitive query semantics land first; the
  selector is main-profile-only; `?bots=1` is removed and ignored; the history
  route is unchanged; broader game-type/history rollout is backlog #59.
