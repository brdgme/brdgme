# 25: Rules Rendering for Humans (Web UI + Email)

**Status:** Pending - post-go-live, non-blocking for cutover (added 2026-07-04)

**Problem:** `game_versions.rules` (one `RULES.md` per game version, added by
migration `rust/web/migrations/004_game_version_rules.sql`) is populated at
deploy/reconcile time and today is consumed by exactly one reader: the bot
(`rust/bot/src/main.rs` selects `gv.rules`, `rust/bot/src/prompt.rs`'s
`PromptContext.game_rules` injects it verbatim into the LLM system prompt).
Human players never see it - the web UI has no rules page or link, and email
(Phase 22) doesn't reference it either. Players have to go read source on
GitHub to learn how to play.

**Sequencing:** post-go-live, after the Phase 16 cutover. The web UI part
depends on nothing else in the backlog and can be built any time after 16.
The email part reuses the 22b `email_render` module (see
`docs/plan/22-email-via-resend.md`), so it must come after 22b lands.
Priority-order placement: "... #22b-d (play-by-email, reminders, multi-email)
→ #24 game invites → #25 rules rendering → #23 Rust game ports (ongoing)."

## Design

### Single source, render-time specialization

No file split. `RULES.md` stays the single source of truth per game version,
stored in `game_versions.rules`, authored per `docs/RULES.md`'s constraints
(no verbatim rulebook copying, comprehensive, source-verified). Nearly all of
its content is equally relevant to a human reading in a browser/email client
and to the bot reading it as LLM context - the only part shaped differently
per consumer is the ```` ```brdgme ```` fenced render blocks (see
`docs/RULES.md`'s "Reading the Display" section and the two fenced examples
in `rust/game/lost-cities-2/RULES.md` lines 109-117), which are raw
`brdgme_markup` text meant to be turned into a styled board render for
humans, but are already perfectly readable as annotated markup by the bot.

Each consumer therefore gets its own render step over the same source text:

- **Bot:** unchanged. `rust/bot/src/main.rs` reads `gv.rules` from the DB and
  `prompt.rs::render_prompt` drops it into the system prompt as raw text -
  no code changes needed here for this phase.
- **Web UI:** render the markdown server-side into HTML, intercepting
  ```` ```brdgme ```` fences and piping their contents through the same
  markup pipeline `rust/web/src/game/server_fns.rs::get_game_details` already
  uses for live game renders (lines 194-209): `brdgme_markup::from_string` →
  `brdgme_markup::transform` → `brdgme_markup::html`.
- **Email (folded into/after 22b):** the same fence-interception step, but
  finishing with the 22b `email_render` module's two outputs instead of the
  web's: `brdgme_markup::html()` (needs inline styles for email-client
  compatibility, matching the legacy dark-terminal `<pre>` styling described
  in 22's "Learnings from legacy Go brdg.me" section) for the HTML part, and
  `brdgme_markup::plain()` for the text/plain part.

### Markdown rendering choice

Use `pulldown-cmark` for the non-fence markdown → HTML conversion, server-side
only (SSR-gated, like the rest of `rust/web/src/game/server_fns.rs`) so it
never touches the WASM client bundle. Verified it is **not** currently a
dependency anywhere in the workspace (`grep -rn "pulldown-cmark\|pulldown_cmark"`
across `rust/**/Cargo.toml` and `*.rs` returned nothing) - this phase adds it
as a new `rust/web` dependency, SSR-feature-gated only.

Pipeline for the web server fn:

1. Split the RULES.md source on ```` ```brdgme ... ``` ```` fences (a small
   scanner, not a full CommonMark-aware split - fences are always on their
   own lines per the `docs/RULES.md` authoring convention, so a line-based
   scan for the fence marker is sufficient and doesn't need a markdown AST).
2. Feed each non-fence chunk through `pulldown_cmark::Parser` +
   `pulldown_cmark::html::push_html` to get its HTML.
3. Feed each fence's contents through `brdgme_markup::from_string` →
   `transform` → `html` (synthetic players, below) and wrap the result in a
   container `<div>`/`<pre>`-equivalent styled the same as the live game
   board render, so a rules render looks identical to gameplay.
4. Concatenate in original order and return one HTML string.

### Synthetic players for `transform`

Rules renders reference `{{player N}}` (see the `{{player 0}}`, `{{player 1}}`,
`{{player 2}}` tokens throughout the fenced examples in
`rust/game/lost-cities-2/RULES.md`), and `brdgme_markup::transform` (in
`rust/lib/markup/src/lib.rs`, re-exported from `crate::transform`) requires a
`&[brdgme_markup::Player]` slice to resolve them - each `Player` is
`{ name: String, color: brdgme_color::Color }`. This mirrors how
`get_game_details` builds `markup_players` today (server_fns.rs lines
198-207): one `brdgme_markup::Player` per real `game_players` row, colour
from `brdgme_color::Color::from_str(&p.game_player.color)`.

For a rules page there is no game, so build a synthetic list instead:
`Player { name: format!("Player {}", i + 1), color: <palette[i]> }` for
`i in 0..max_player_count`, where `max_player_count` is `max()` of the game
type's `player_counts` (a **set** of supported counts, e.g. `{2, 4}`, not a
min/max range - decided 2026-07-05: always size to the largest supported
count). A `{{player N}}` token in a fence that falls outside that range is
treated as an authoring error, not something the renderer needs to tolerate
- `docs/RULES.md`'s process step 5 already requires verifying command/render
content against source, so an out-of-range player reference should fail
loudly (render error or panic-free `Result` error surfaced to the page) and
get caught in review, not silently degrade. Use the same 7-colour palette
`rust/web/src/db.rs` assigns real games at creation time (line 703-705:
`["Green", "Red", "Blue", "Amber", "Purple", "Brown", "BlueGrey"]`, parsed via
`brdgme_color::Color::from_str`) so a rules render's colours match what
players will actually see in a real game.

### Server fn shape

New server fn, e.g. `get_rendered_rules(game_version_id: Uuid) -> Result<String, ServerFnError>`,
alongside the existing ones in `rust/web/src/game/server_fns.rs`. Input is
just the version id (rules are keyed 1:1 to `game_versions`, per
`rust/web/src/db.rs`'s `find_game_version`/`find_latest_non_deprecated_game_version`,
lines 199-236 - note neither currently `SELECT`s the `rules` column, only
`id, created_at, updated_at, game_type_id, name, uri, is_public, is_deprecated`).
Decided 2026-07-05: add a narrow dedicated query,
`find_game_version_rules(pool, id) -> Option<String>`, rather than widening
`GameVersion`'s `sqlx::query_as!` - keeps the (potentially large) rules text
out of every other call site that fetches a `GameVersion` today. Output is a
single sanitized HTML string, consistent with how
`get_game_details` delivers `html` for game boards - the Leptos component
consuming it can use the same `GameBoard`-style raw-HTML injection point
already in `rust/web/src/components/game.rs` (referenced from `app.rs`'s
`GamePage` as `<GameBoard html=html />`).

**Sanitization:** rules markdown is deploy-time trusted content, authored
in-repo per `docs/RULES.md` and populated into the DB only by the operator
reconcile step described there ("The operator populates this column by
calling `Request::Rules` against the game service during reconcile") - not
user input. No HTML sanitization pass is required beyond what
`pulldown-cmark` and the existing `brdgme_markup::html` renderer already
produce; treat it the same trust level as the game-render HTML pipeline it
reuses.

### Routing / UI placement

Current routes (`rust/web/src/app.rs::App`, lines 51-59): `""` (home),
`"login"`, `"games"` (new-game form, `GamesPage`), `"dashboard"`, and
`("games", ParamSegment("id"))` (an existing game, `GamePage`). There is no
existing "browse game types" page - `GamesPage` is the create-game form,
fetching `GameTypeInfo`/`GameVersionInfo` via `get_available_game_types`
(server_fns.rs ~line 293) which already carries `game_version.id` client-side.

Decided 2026-07-05: a standalone `("rules", ParamSegment("version_id"))` →
`RulesPage` route, linked from two places that already have a
`game_version_id` in scope (no inline collapsible panel on `GamePage` - a
separate routed page is simpler, works before a game exists, and is
shareable/linkable, which an in-game-only panel would not be):
- The new-game form (`GamesPage` in `app.rs`) - a "View rules" link next to
  the version selector, using `selected_version_id`.
- The in-game view (`GamePage`) - a "Rules" link in the game meta/sidebar
  (`GameMeta` component), using `ge.game_version.id` (already loaded into
  `GameViewData`, though not currently exposed as a field - would need to
  thread `version_id` through `GameViewData` alongside `version_name`, or
  have the page derive it from data already sent).

This keeps rules addressable by version id directly (shareable link, works
before a game exists) rather than nesting it under a specific game.

### Caching

Rules are immutable per game version once written by the reconcile step
(RULES.md is authored per-version; a rules edit for an existing version isn't
part of the authoring workflow in `docs/RULES.md` - a new version is created
instead). Rendering per-request is fine as a v1 (pulldown-cmark + the markup
pipeline are cheap, matching cost order with live game render calls already
done per-request in `get_game_details`). If it becomes a hot path, memoizing
rendered HTML keyed by `game_version_id` (in-process `HashMap` behind a
`Mutex`/`OnceCell`, or a small DB column caching the rendered HTML) is a
straightforward follow-up - not needed for v1.

### Email integration (after 22b)

Small, explicitly deferred until 22b's `email_render` module exists. Decided
2026-07-05, both now committed scope for this phase (not optional):
- **Invite/notification emails** (22, `docs/plan/24-game-invites.md`) get a
  "View rules" link back to the web `/rules/{version_id}` page - link-only,
  not full rules content inlined in every mail, keeping mail size down and
  reusing the same rendered page.
- **A `rules` reply command** (alongside 22b's other server commands -
  `concede`, `undo`, `restart`, `unsubscribe`/`subscribe`) replies with the
  full rendered rules (text + HTML) via `email_render`, for players who want
  them inbox-side without leaving their email client - consistent with 22b's
  "full interface" goal.

Neither blocks the web UI half of this phase, and the web UI half doesn't
block them - both simply cannot start until 22b's `email_render` module
exists.

### Future escape hatch (documented, not built)

If genuinely bot-only content is ever needed in a RULES.md (content that
should influence the LLM but never render to a human), an HTML-comment
marker convention - e.g. `<!-- bot-only -->...<!-- /bot-only -->` - stripped
by the human-facing renderers (web + email) before the markdown/fence
pipeline runs, covers it without splitting files. The bot's raw-text
consumption is untouched either way. Nothing in any existing RULES.md
requires this today; it's a documented option only.

## Tasks

- [ ] Add `pulldown-cmark` as an SSR-only dependency of `rust/web` (verify
      workspace dependency conventions in `rust/web/Cargo.toml` first - other
      SSR-only deps are feature-gated the same way `resend-rs`/`sqlx` are).
- [ ] DB: `find_game_version_rules(pool, id) -> Option<String>` narrow
      dedicated query in `rust/web/src/db.rs` (decided 2026-07-05: do not
      widen `GameVersion`'s existing `sqlx::query_as!`).
- [ ] `brdgme`-fence scanner + markdown renderer (small module, e.g.
      `rust/web/src/game/rules_render.rs`): split source on fences, render
      non-fence chunks with `pulldown-cmark`, render fence contents through
      `brdgme_markup::from_string` → `transform` → `html` with synthetic
      players, concatenate.
- [ ] Synthetic player list helper (name `Player {N}`, colour from the
      existing 7-colour palette in `rust/web/src/db.rs` lines 703-705 -
      consider extracting that array to a shared location if both call
      sites need it, avoiding duplication).
- [ ] Server fn `get_rendered_rules(game_version_id: Uuid)` in
      `rust/web/src/game/server_fns.rs`.
- [ ] `RulesPage` Leptos component + route in `rust/web/src/app.rs`
      (`("rules", ParamSegment("version_id"))`), reusing the raw-HTML
      injection pattern from `GameBoard`.
- [ ] Links to the rules page from `GamesPage` (new-game form) and
      `GamePage`/`GameMeta` (in-game view); thread `version_id` through
      `GameViewData` if not already available where needed.
- [ ] Tests (Phase 11 patterns, `rust/web/src/game/server_fns.rs` /
      `rust/web/tests`): a golden test feeding a markdown + `brdgme`-fence
      fixture through the renderer and asserting both the markdown HTML and
      the fence's rendered board HTML appear correctly ordered and
      well-formed; a `{{player N}}` resolution test against the synthetic
      player list; an SSR page test for the new route following the 11.6a
      pattern in `rust/web/tests/ssr_pages.rs`.
- [ ] Email tasks (after 22b, small, both committed scope): rules link in
      invite/notification mail; `rules` reply command via `email_render`.

## Decisions (resolved 2026-07-05)

- **Query shape:** narrow dedicated query, `find_game_version_rules(pool,
  id) -> Option<String>`. Does not widen `GameVersion`'s existing
  `sqlx::query_as!` - keeps rules text out of every other call site that
  fetches a `GameVersion` today.
- **Synthetic player count:** size the list to `max()` of the game type's
  `player_counts` set (e.g. `{2, 4}` → 4 synthetic players), not a fixed cap.
  A `{{player N}}` reference outside that range is an intentional authoring
  error signal - it should fail loudly rather than be tolerated, matching
  `docs/RULES.md`'s existing source-verification discipline.
- **UI placement:** standalone `/rules/{version_id}` page, linked from the
  new-game form and the in-game view. No inline collapsible panel on
  `GamePage`.
- **Email:** link-only rules reference in invite/notification mail (no full
  rules inlined), plus the `rules` reply command is committed scope for this
  phase (not optional) - replies with the full rendered rules via
  `email_render`.

## Verified-against-source notes

- `GameVersion`'s existing `sqlx::query_as!` calls in `rust/web/src/db.rs`
  (`find_game_version` lines 199-215, `find_latest_non_deprecated_game_version`
  lines 218-236) do **not** currently select `rules` - confirmed by reading
  both queries. The new `find_game_version_rules` query (decided above) adds
  the needed path without touching these.
- `pulldown-cmark` is confirmed absent from the workspace today (searched all
  `Cargo.toml` and `*.rs` under `rust/`).
- The bot's rules consumption (`rust/bot/src/main.rs` line 89 SQL, line 117
  `try_get("rules")`, `prompt.rs::PromptContext.game_rules`) needs no changes
  for this phase - included here only as the "unchanged" leg of the
  render-time-specialization design.
- No operator change is needed to populate `rules`: `rust/operator/src/
  controller.rs` lines 112-120 already calls `Request::Rules` against the
  game service during reconcile, and lines 122-187
  (`upsert_game_type_and_version`) already upserts the result into
  `game_versions.rules` with `rules = EXCLUDED.rules` on conflict (line 174).
  This phase only adds readers, not writers.
