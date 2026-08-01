# 29: Player Stats and Historical Reports - Design

> Extracted 2026-07-08 from `docs/plan/29-stats-reports.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Draft - brainstormed and scoped 2026-07-08. Post-go-live,
non-blocking. No schema changes required for v1; everything is derived
from existing data.

**Problem:** brdg.me has no historical view of finished games. You
cannot look up how you (or anyone else) have been doing recently, see
rating progression, or compare records against other players - even
though the database already stores everything needed to derive all of
this.

## Data inventory (audited 2026-07-08)

All v1 features are pure derivations from existing tables. No
migrations, no new write paths.

| Source | Enables |
|---|---|
| `game_players.place` | wins, win %, average placing, placing histogram |
| `game_players.rating_change` + `games.finished_at` | ELO-over-time series per game type |
| `game_players.points` | average/best score per game type |
| `game_type_users.rating`, `peak_rating` | current + peak rating, leaderboards |
| `games.created_at` / `finished_at` | games per month, activity over time |
| `game_players.user_id` joined per game | head-to-head records vs specific opponents |
| `game_players.position` + `place` | seat/first-player advantage per game type |
| `game_players.is_eliminated` | elimination rate |
| `game_logs.logged_at` | game length in turns, turn pace |

Key facts:

- **ELO history is fully reconstructible.** Legacy `rust/api` has
  always written `game_players.rating_change` on finish and #12 ported
  that to `rust/web`. Per-user-per-game-type rating over time is
  `1200 + cumulative sum of rating_change ordered by finished_at`.
  Sanity check during implementation: reconstructed final value should
  equal `game_type_users.rating`; investigate any drift.
- **`peak_rating` is historically wrong.** Legacy never updated it
  (#12); the column default is 1200 for everyone. Backfill it from the
  reconstructed series as part of this phase (one-off UPDATE, cheap).
- **`users.name` is UNIQUE**, so name-based profile URLs work without
  exposing UUIDs.
- **Ties:** `place = 1` can be shared. Definition: a win is
  `place = 1`, shared or not.

## Decisions (2026-07-08)

- **D1 - bot-game inclusion rule.** A finished game counts toward
  play/win/placing stats iff it has >= 2 human players; bots present or
  not is irrelevant. Games with exactly 1 human (human-vs-bots-only)
  are excluded by default, with a user-facing toggle on stats pages to
  include them (default off). This does NOT change the #12 rating rule:
  any game containing a bot is never rated, so bot-inclusive games
  contribute to counts/placings but never to ELO.
- **D2 - zero-dependency charting.** No charting library, no JS. Three
  tiers, all SSR:
  1. Colored text (existing `.brdgme-*` classes) for form strips and
     rating deltas.
  2. Unicode block sparklines (`U+2581`-`U+2588`) in a span for tiny
     inline trends - the site is already fully monospace, so text
     sparklines look native.
  3. Server-rendered inline SVG from Leptos `view!` for the ELO line
     chart and histograms: map points to viewBox coordinates, emit
     `<polyline>` / `<rect>`, native `<title>` elements for hover
     tooltips. Adds nothing to the WASM bundle, renders in initial
     HTML, styled via CSS with the existing palette.
- **D3 - derive, don't denormalize.** v1 computes stats with SQL
  aggregate queries at request time. Game volume is low; no
  materialized views or stats tables until a real performance problem
  appears.

## Awareness (explicitly not planned now)

- **Cooperative games** are a future addition and cannot use pairwise
  ELO. Some game types will one day opt out of rating entirely (or use
  a different scheme). Nothing in this phase should hard-assume every
  game type is rated - treat "no rating data" as a valid state when
  rendering, which the bot-game rule already forces anyway.

## v1 scope: pages / routes

- **`/players/:name`** - user report (viewable for any user, not just
  self):
  - Header: name, member since, total finished games, overall wins /
    win %.
  - Per-game-type rows: current rating, peak rating, games played,
    win %, average placing, inline sparkline of recent rating trend,
    form strip.
  - Recent finished games list (x most recent): game type, date,
    placing, rating change, opponents (linking to their profiles).
  - Active games list when viewing your own profile.
  - Bot-games toggle (D1), off by default - query param so it is
    linkable.
- **`/players/:name/:game_type`** - per-game-type deep dive:
  - ELO-over-time SVG line chart (D2 tier 3).
  - Placing histogram, bucketed by player count (2p / 3p / 4p+) - win %
    across mixed player counts is misleading, buckets fix that.
  - Full finished-games list for that type with placings and rating
    changes.
  - Head-to-head table: record vs each opponent faced in this game
    type.
- **Form strip embedding** - last 5-10 results as colored characters
  (`W`/`L` for 2p, placing digits for multiplayer; green for wins, red
  for losses via existing classes). Embed in:
  - Game meta panel next to each player name (form in the current game
    type).
  - Player report pages.

## Stats definitions

- Win: `place = 1` (shared firsts count).
- Win % denominator: finished games passing the D1 inclusion rule.
- Average placing normalized across player counts where a single
  number is wanted: percentile `(n - place) / (n - 1)`.
- Form strip: most recent first or last (pick one, be consistent,
  label it).

## v2 candidates (not scoped)

- Global per-game-type page (`/stats/:game_type`): site-wide game
  count, rating leaderboard with peaks, seat-order win rates, average
  game length in turns.
- Rivalry view: record vs one opponent across all game types; most
  played opponent, nemesis (worst win rate against), favorite victim.
- Streaks: current / longest win streak per game type.
- Biggest upset: largest single-game `rating_change`.
- Activity chart: games finished per month.
- Turn pace: median `is_turn_at` -> move time from `game_logs`. Noisy
  for async play; lowest priority.
