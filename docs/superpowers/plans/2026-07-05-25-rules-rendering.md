# 25: Rules Rendering for Humans (Web UI + Email) - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/25-rules-rendering.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Spec:** `docs/superpowers/specs/2026-07-05-25-rules-rendering-design.md`

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
