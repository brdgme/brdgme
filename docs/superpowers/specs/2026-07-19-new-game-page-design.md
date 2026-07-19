# New Game Page Rebuild - Design

Date: 2026-07-19
Status: approved for planning (expect iteration during development)
Related backlog: #44 ("New game" screen usability). Related but out of scope:
#24 (invites instead of auto-start), #22 (play-by-email), bots-in-DB work.
Companion spec: `2026-07-19-new-game-preview-design.md` (severable, separate
plan; this page must be complete and good without it).

## Problem

The current `/games` page (`GamesPage`, `rust/web/src/app.rs:452-763`) hides
39 games in a `<select>`, offers no supplemental information (player counts,
weight, description), and has essentially zero bespoke CSS. Users bounce off
it. The rebuild must make browsing games compelling while retaining the
site's simplicity.

## Constraints

- Standard HTML inputs only. No images, no animations, no custom widgets
  replacing native form controls.
- Fully theme-compatible: colors only via `--mk-*` custom properties or
  `mk-fg-*`/`mk-bg-*` classes; must work under all 33 themes; no meaning
  encoded in hue alone; translucency via `color-mix(... transparent)`.
- Desktop may use multiple columns; tablet and mobile collapse to a usable
  single column. Follow existing breakpoints (80em sidebar collapse, 60em).
- Stays within the `.content-page` wrapper (max-width 1220px, centered).

## Flow

Game first, players second. The user browses/filters the game grid, selects
a game, and the setup form appears constrained to that game's player counts.

## Layout

Two-pane on desktop (>= ~60em): game grid left, detail/setup panel right.
The panel is sticky so it stays visible while the grid scrolls. Below the
breakpoint: single column - filters, grid, then the detail/setup form after
the grid; selecting a game auto-scrolls the form into view.

```
DESKTOP
+---------------------------+------------------+
| [players] [search] [sort] |  Acquire         |
| +-------+ +-------+       |  2-6 players     |
| |Acquire| |Age of |       |  Weight 2.5 / 5  |
| |2-6p   | |Steam  |       |  blurb text...   |
| +-------+ +-------+       |  Version: [v]    |
| +-------+ +-------+       |  Players: (2)(3) |
| |Brass  | |Can't  |       |  Opponents:      |
| |       | |Stop   |       |   [slot 1]       |
| +-------+ +-------+       |   [slot 2]       |
|  ... grid scrolls ...     |  [Start game]    |
+---------------------------+------------------+
```

## Game grid

- Each game is a `<label>` card wrapping a visually-hidden-but-accessible
  `<input type="radio" name="game-type">`. Native keyboard and screen-reader
  semantics; selected state styled from `:checked`.
- Card contents: game name, player range (e.g. "2-6 players", honoring
  non-contiguous counts), weight ("Weight 2.5 / 5"), 1-2 sentence blurb.
  Empty blurb renders nothing.
- Multi-column via CSS grid (`auto-fill`/`minmax`), collapsing to one column
  on narrow screens.

## Filters and sort (client-side, over the already-fetched list)

- Player count filter: hides games whose `player_counts` do not include the
  entered count. Cleared = show all.
- Text search: plain text input filtering by name as you type.
- Sort: `<select>` - Alphabetical (default), Weight (low to high), Weight
  (high to low).
- Filtering out the currently selected game deselects it and returns the
  panel to its empty state.

## Detail / setup panel

- Empty state before any selection: short prompt to pick a game.
- After selection: name, player range, weight, blurb; then the form:
  - Version `<select>` - rendered only when the game has more than one
    version (most have one).
  - Player count as radios, options exactly the game's `player_counts`;
    default to the lowest.
  - Opponent slots, auto-sized to `player_count - 1` (existing behavior).
  - Start game button; on success, redirect to `/games/{id}` (unchanged).

## Opponent slots

Per-slot mode radio: Player / Email / Bot. The chosen mode reveals its
input:

- **Player**: typeahead text input searching all users by display name via a
  new server fn. Friends and recent opponents (existing
  `get_opponent_suggestions()`) shown as suggestion chips before typing.
  Selecting a match or chip fixes the slot to that user id.
- **Email**: plain `<input type="email">`, required.
- **Bot**: name input (default "Bot {n}") and difficulty `<select>`
  (easy/medium/hard). Difficulty remains a string end to end for now; the
  enum/validation belongs to the upcoming bots-in-DB work.

## Backend changes

1. **Blurb field**, following the existing `weight` pattern end to end:
   - `GameVersionSpec` field in `rust/operator/src/crd.rs` (optional,
     defaults to empty).
   - Regenerate `k8s/base/operator/crd.yaml`.
   - Add blurbs to the 39 per-game YAMLs in `k8s/base/game/*/game-version.yaml`.
   - Migration: `ALTER TABLE game_types ADD COLUMN blurb TEXT NOT NULL
     DEFAULT ''` (numbered per `migrations/` convention).
   - Operator upsert (`rust/operator/src/controller.rs`), model
     (`models/game.rs`), queries (`db.rs`).
2. **Expose in `GameTypeInfo`** (`rust/web/src/game/server_fns.rs`): add
   `weight` and `blurb` (both already/soon in `game_types` but not exposed).
3. **Roster validation**: `create_new_game` rejects requests where
   `1 + opponents` is not in the game's `player_counts`.
4. **User search server fn**: search users by display name; login required;
   minimum 2 characters; results capped (e.g. 10); excludes the current
   user.

## Content

Claude drafts 1-2 sentence blurbs for all 39 games; user reviews and edits
before merge.

## Error handling

- Game list load failure: existing page-level error handling.
- User search failure: show inline error under the slot; slot remains
  usable via Email mode.
- Server-side roster validation failure surfaces as a form error (should be
  unreachable through the constrained UI).

## Testing

- Server fn tests: roster validation (accept/reject), user search (min
  length, cap, excludes self), `GameTypeInfo` carries weight/blurb.
- Operator: blurb upsert covered alongside the existing weight path.
- UI verified manually across themes and breakpoints (existing convention;
  no browser test harness in repo).

## Out of scope

- Board preview (companion spec).
- Rules display on this page (depends on a future rules-rendering pass).
- Bot difficulty enum / bots-in-DB.
- Game invites (#24), play-by-email (#22).
