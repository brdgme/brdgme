# New Game Page - Board Preview - Design

Date: 2026-07-19
Status: approved for planning (expect iteration during development)
Companion to: `2026-07-19-new-game-page-design.md`. Strictly severable: the
new game page must be complete and fully functional if this feature is
absent, disabled, or failing.

## Goal

When a game is selected on the new game page, show its initial board render
in the detail panel so users can see what the game looks like before
starting one.

## Approach

Server-cached fresh-board render:

- On first request for a game version, the web server calls that version's
  microservice with `Request::New` using a fixed seed and placeholder player
  names (the existing stateless path used by `create_game_from_service`,
  `rust/web/src/game/server_fns.rs:320-373` - no DB rows created).
- The returned markup is converted to HTML via the existing pipeline
  (`brdgme_markup`: `from_string` -> `transform_semantic` -> `html_class`),
  with the standard inline `--mk-player-{n}` vars so it themes correctly
  (same isolated-preview pattern as the theme picker tiles,
  `rust/web/src/theme.rs:210-219`).
- The render is deterministic per version (fixed seed), so it is cached
  in-memory keyed by game version id and reused for all users. The cache
  repopulates lazily after restarts. No DB changes.
- Player names in the preview are generic placeholders; player count for the
  preview is the game's lowest supported count.

## Delivery to the client

New server fn `get_game_preview(game_version_id)` returning the cached HTML
(or an error). The detail panel fetches it lazily when a game is selected
and shows a lightweight "Loading preview..." placeholder meanwhile.

## Size handling

Renders vary wildly; some (e.g. Cathedral) are large in BOTH width and
height.

- The preview sits in a container with a fixed max-height and
  `overflow: auto` in both axes - it must never widen the panel or the
  page.
- Rendered at reduced scale (~50%) via a font-size reduction on the
  container (renders are text-based, so this scales cleanly without
  transforms).
- Exact scale and max-height tuned during development against the largest
  renders (Cathedral) and the smallest.

## Failure behavior

Any failure (microservice unreachable, render error, markup transform
error) results in the preview area simply not rendering. No error surfaced
to the user beyond the absence of a preview; the rest of the panel and form
are unaffected. Failures are not cached, so a later selection retries.

## Testing

- Server fn: cache hit path, placeholder-player construction, failure path
  returns error without poisoning the cache.
- Manual verification across several games including Cathedral (largest)
  and a small render, in light and dark themes.

## Out of scope / future

- Mid-game snapshots (bots playing a few turns for a livelier board) -
  possible later enhancement via the operator capture path.
- Representative real finished games (dropped: privacy, new queries,
  unpredictable content).
- Rules display (separate future pass on rules rendering).
