# 26: Web Chrome Theming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Created:** 2026-07-13
**Parent plan:** `2026-07-13-26-theming-semantic-colors.md` (this closes out its D11)
**Goal:** All web chrome in `rust/web/style/main.scss` (nav, sidebar, buttons,
forms, panels, borders) themes correctly under brdgme light, brdgme dark, and
Dracula via the existing `--mk-*` CSS custom properties. Instant switching
keeps working (pure CSS var swap - no new JS needed).

**Architecture:** main.scss's hardcoded colours are replaced with
`var(--mk-*)` references. Chrome surface tints reuse the palette `soften`
machinery: a new `CHROME_SOFTENS` list in `rust/web/src/theme.rs` is
concatenated with `IN_USE_SOFTENS` when calling `palette_css_vars`, so chrome
tints derive from the theme exactly like game surfaces. Translucent overlays
use CSS `color-mix(in srgb, var(--mk-...) N%, transparent)` (no palette
change needed). The dead `.brdgme-*` classes are deleted.

**Verify with:** `SQLX_OFFLINE=true cargo check -p web --features ssr`,
`... --features hydrate`, `SQLX_OFFLINE=true cargo test -p web --lib`
(DB sqlx::tests will fail on missing database - expected, ignore),
`cargo test -p brdgme-color` (contrast gate), `cargo fmt --check -p web`.

## Colour mapping (the whole change surface in main.scss)

| Current hardcoded | Sites | Replacement |
|---|---|---|
| `#fff` body bg | body | `var(--mk-background)` |
| `#606060` text | body, spinner dots, header-icon-button, menu h1/subheading links, layout-game links | `var(--mk-grey)` |
| `blue` links | a | `var(--mk-blue)` |
| `#e0e0e0` surfaces/borders | layout-header, menu bg, game-main border-right, recent-logs-header bg, recent-logs border, suggestions border, disabled input bg, game-logs-summary borders, game-meta bg, game-current-turn bg+border, theme-tile border | `var(--mk-soften-foreground-86)` |
| `#feedc3` my-turn | layout-header.my-turn, layout-game.my-turn | `var(--mk-soften-orange-86)` |
| `#f7d7d7` finished | layout-game.finished | `var(--mk-soften-red-86)` |
| `#fafafa` hover | layout-game:hover | `var(--mk-soften-foreground-96)` |
| `rgba(255,255,255,0.63)` underlays | menu-close-underlay, game-meta-close-underlay | `color-mix(in srgb, var(--mk-background) 63%, transparent)` |
| `rgba(255,255,255,0.85)` | game-logs-summary bg | `color-mix(in srgb, var(--mk-background) 85%, transparent)` |
| `rgba(224,224,224,0.6)` | game-logs-summary .header bg | `color-mix(in srgb, var(--mk-soften-foreground-86) 60%, transparent)` |
| `#d32f2f` | command-error, rating-change-down | `var(--mk-red)` |
| `#388e3c` | rating-change-up | `var(--mk-green)` |
| `#1976d2` | rating-change-none | `var(--mk-blue)` |
| `#909090` | game-logs .log-time | `var(--mk-grey)` |
| `#fff` | game-meta-logs-content bg | `var(--mk-background)` |
| `.brdgme-*` block (lines 534-555) | dead code per THEMING.md Appendix B audit | DELETE |

## Rust-side changes

- `rust/web/src/theme.rs`: add
  `const CHROME_SOFTENS: &[(NamedColor, u8)] = &[(Orange, 86), (Red, 86), (Foreground, 96)];`
  and in `build_theme_style_css()` pass `IN_USE_SOFTENS ++ CHROME_SOFTENS`
  (concatenated Vec) to every `palette_css_vars` call. Keep the existing
  `body{...}` line in `build_theme_style_css` (harmless; main.scss now agrees
  with it). Update `theme_style_css_contains_expected_rules` to assert
  `--mk-soften-orange-86` is present.
- `rust/lib/color/` NOT changed (IN_USE_SOFTENS stays games-only; the
  contrast gate's scope is game text, chrome text on chrome tints is
  foreground/grey which the gate already validates against soften surfaces
  of the same construction).
- New web unit test: for each registered theme, assert foreground reaches
  3:1 WCAG contrast against each CHROME_SOFTENS-derived surface (use
  brdgme_color's soften + contrast-ratio helpers). This is the chrome
  contrast gate.

## Orchestration

Serial sub-orchestrators (one at a time), implementation subagents on
Sonnet 5, per the standing process. Top-level session only reads files and
updates this plan.

| # | Orchestrator scope | Verify |
|---|---|---|
| A | theme.rs CHROME_SOFTENS + main.scss full migration per table + delete .brdgme-* + chrome contrast test + updated unit tests | web ssr+hydrate checks, web --lib tests, brdgme-color tests, fmt |
| B | Independent review sweep: grep main.scss for any remaining hex/rgb/named colours (only expected leftovers: none), confirm var names all exist in THEME_STYLE_CSS output, re-run all verify commands, eyeball SCSS diff for selector regressions. ALSO fix clippy single_match at lib/markup/src/parser.rs:560 (test helper: `match result { Ok((nodes, _)) => { assert!(...) } Err(_) => {} }` -> `if let Ok((nodes, _)) = result { assert!(...) }`), then `cargo clippy -p brdgme-markup -- -D warnings` + `cargo test -p brdgme-markup` | all green |

## Decisions made without user input (for post-completion review)

- E1: my-turn `#feedc3` -> `soften(orange, 86)` (light: ~#ffe4c4 tone vs the
  old warmer amber tint) and finished `#f7d7d7` -> `soften(red, 86)` - small
  hue drift accepted to stay on-palette rather than adding bespoke slots.
- E2: `#606060` chrome text -> GREY slot (light #616161, 1/255 drift);
  `#909090` log-time also folds to GREY (visibly darker than before) rather
  than adding a new soften-for-text (softened colours are backgrounds-only
  per THEMING.md).
- E3: Translucent overlays use `color-mix(...)` - requires a 2023+ browser;
  accepted (Leptos/WASM already requires modern browsers).
- E4: Chrome softens live in web/theme.rs, not IN_USE_SOFTENS, so the
  brdgme-color contrast gate's game-text scope is unchanged; a web-side test
  gates foreground-on-chrome-tint instead.
- E5: hover `#fafafa` -> `soften(foreground, 96)` (new pct 96; pct is a free
  integer per D5).
- E6: dead `.brdgme-*` classes deleted outright (audit found no users).

## Progress log

- [x] Orchestrator A: chrome migration (2026-07-13): main.scss fully migrated
  per mapping table, .brdgme-* block deleted; theme.rs CHROME_SOFTENS
  concatenated with IN_USE_SOFTENS at all three palette_css_vars sites;
  new chrome_softens_meet_contrast_floor test (3.0 WCAG floor, all themes,
  green); brdgme_color::contrast_ratio made pub (was private). Verified:
  web ssr+hydrate checks pass; web --lib --features ssr 16 pass (52 DB
  sqlx failures expected, no DB); brdgme_color 11 pass; fmt clean; grep
  clean. Gotcha: `cargo test -p web --lib` needs `--features ssr` (dev
  deps ssr-gated); crate name is brdgme_color (underscore).
- [x] Orchestrator B: review sweep (2026-07-13): main.scss diff matches the
  mapping table line-by-line, colour-only changes, zero remaining colour
  literals; all 10 --mk-* vars used are emitted by THEME_STYLE_CSS
  (NamedColor::ALL + IN_USE_SOFTENS + CHROME_SOFTENS); parser.rs:557
  single_match -> if let fixed. Verify all green: web ssr/hydrate checks,
  web --lib --features ssr (16 pass, 52 sqlx-DB failures expected),
  brdgme_color 11, brdgme_markup 36, clippy -D warnings on brdgme_markup
  + web ssr, workspace fmt --check. Minor pre-existing: main.scss missing
  trailing newline at EOF. Phase complete; uncommitted.
