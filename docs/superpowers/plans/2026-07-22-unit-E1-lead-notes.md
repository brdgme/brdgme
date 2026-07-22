# E1 Lead notes - stepped routes + type-selection page

Lead: E1. Status: DONE. Commit `aa8d24814d1c9042cbb2f5ea3663a1701f944c26` (not pushed).
Gates green: fmt, clippy -p web --all-targets --features ssr -D warnings, check --features hydrate, test -p web --features ssr (394 lib + 38 ssr_pages).

## Resolved design (brief overrides plan doc D-route)
- `/games` left UNUSED (no redirect/alias). Comment out the route, reserved.
- `/games/new` -> `NewGameTypePage` (full-width game-card grid; cards are `<A>` links to `/games/new/{encode_path_segment(name)}`).
- `/games/new/:type` -> `NewGameSetupPage` (moved GameBrowser setup form, reads `:type` path param, case-insensitive name match, "Game not found" for unknown).
- Both pages use `LocalResource` (existing pattern; None on SSR -> "Loading..." shell). Hydration-safe by construction.
- GameInfoPage start href: `/games?game={name}` -> `/games/new/{encode_path_segment(name)}`.
- Sidebar "New game" link: `/games` -> `/games/new`.

## new_game.rs split
- `NewGameTypePage` (pub) = old NewGamePage shell + `GameTypeGrid`.
- `GameTypeGrid` = filters + `.game-card-grid` of `<A attr:class="game-card">` links.
- `NewGameSetupPage` (pub) = "Back to games" link (outside resource = SSR marker) + resolve type + `GameSetupPanel` | "Game not found".
- `GameSetupPanel` = old GameBrowser right column, signals seeded from `gt` prop.
- Removed: `is_narrow`, `select_game`, `panel_ref`, `?game=` preselect effect, `use_query_map`.

## Routes (app.rs) - place before /games/:id
- `(games, new)` -> NewGameTypePage
- `(games, new, :type)` -> NewGameSetupPage
- comment out `(games)` NewGamePage route (reserved)

## Tests (ssr_pages.rs)
- `games_page_anonymous` -> `new_game_type_page_anonymous`: GET /games/new, marker "New Game", assert !contains("Invalid Game ID") [G-route proof].
- NEW `games_route_is_unused_returns_not_found`: GET /games, marker "Page not found.".
- `new_game_page_with_game_query_param_renders_shell` -> `new_game_setup_page_renders_shell`: GET /games/new/Some%20Game, marker "Back to games".
- NEW `new_game_setup_page_unknown_type_renders_shell`: GET /games/new/NoSuchGame, marker "Back to games".
- `game_info_page_renders_for_existing_game_type`: `/games?game=` -> `/games/new/`.

## G-route proof
GamePage uses new_blocking; for /games/new (non-UUID) SSR renders "Error: Invalid Game ID".
Type page (LocalResource) SSR renders h1 "New Game". Asserting "New Game" present + "Invalid Game ID" absent proves static-beats-param.

## Deviation to report
Brief suggested grid marker `game-card-grid` for /games/new SSR test; with LocalResource (established pattern) the grid is client-rendered, so SSR marker is the h1 "New Game". G-route requirement still met.

## Test env (containers already running)
DATABASE_URL=postgres://postgres:postgres@localhost:15432/brdgme
NATS_URL=nats://localhost:14222
SQLX_OFFLINE=true RUST_MIN_STACK=8388608
run from rust/: cargo test -p web --features ssr
