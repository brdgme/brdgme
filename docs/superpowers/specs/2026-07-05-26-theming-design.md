# 26: Theming / Dark Mode (Web UI + Email) - Design

> Extracted 2026-07-08 from `docs/plan/26-theming.md` (superpowers layout
> migration). Content dates from 2026-07-05; this is a point-in-time decision
> record, not a living document.

**Status:** Pending - post-go-live, non-blocking
**Added:** 2026-07-05

## Decisions (2026-07-05)

- **Board colours: full remap per theme.** Each theme provides values for
  the entire named palette (the ~21 Material colours in `brdgme-color`)
  plus the player-colour sequence. No fixed-board shortcut.
- **Neutral light/dark are the defaults**, mapped from the existing
  Material palette. System `prefers-color-scheme` picks between them when
  the user has no explicit preference. Dracula is opt-in, not the dark
  default (revisit after seeing it live).
- **Persistence: cookie + profile sync.** Theme choice is stored in a
  cookie so SSR renders the right theme with no flash, works logged-out,
  and syncs to the user profile on login. Profile wins on new devices and
  drives email rendering.
- **Emails are themed fully** (chrome + board colours) using the profile
  theme, rendered as inline styles at send time. Client dark-mode
  colour inversion (Gmail/Outlook) is accepted as out of our control.
- **Email tooling: `mrml` + markup inline output (2026-07-05).** Email
  chrome/layout is authored as MJML compiled by the `mrml` crate; the
  board fragment is the markup renderer's existing inline-styled HTML
  resolved against the user's `Palette`, embedded via `mj-raw`. No
  hand-rolled email HTML; `css-inline` deferred unless needed.
- **Custom user themes are a design constraint (2026-07-05).** Not
  needed out of the box, but the low-level design must support users
  defining their own colour codes. Consequence: a theme is *data*, not
  code - `Palette` is a plain runtime value (built-in themes are just
  predefined `Palette` constants), user custom palettes are stored
  per-user (JSON column), and the web pipeline emits per-theme CSS
  custom properties generated from a `Palette` at SSR time rather than
  hand-written per-theme stylesheets. No `enum Theme` anywhere in the
  render path.

## Core architectural change: semantic colours in brdgme markup

This is the load-bearing piece and should land first.

### Problem

Game render colours are baked in as literal RGB:

- `brdgme_markup::ast::ColType` is `RGB(Color) | Player(usize)`. Games
  reference `brdgme_color` statics (`RED`, `BLUE`, ...) which are
  converted to concrete RGB at markup-emit time via `From<Color> for Col`.
- The transform pass resolves `Player(n)` and `Mono`/`Inv` transforms
  down to `TNode::Fg(Color)/Bg(Color)`, and `html.rs` emits inline
  `style="color:#..."`. By the time HTML exists, all semantic intent
  (this is "red", this is "player 2") is gone, so nothing can be
  re-themed.

### Direction

Make colour semantic end-to-end and resolve to concrete values only at
the final render boundary, against a theme palette:

1. **`brdgme-color`:** introduce a `NamedColor` enum covering the current
   palette (Red, Pink, Purple, ... White, Black) and a `Palette` type:
   `NamedColor -> Color` plus the player-colour list and base
   surface/text colours. The current Material values become the built-in
   `LIGHT` palette.
2. **`brdgme-markup`:** add `ColType::Named(NamedColor)`, keeping
   `RGB` as a deprecated escape hatch. Migrate the games (Rust and the
   markup emitted by Go games - audit what the Go side emits; if it
   already emits named colours in markup text like `{{#fg red}}`, the
   parser maps names to `Named` instead of resolving to RGB at parse
   time, and Go needs no changes).
3. **Render-time palette resolution.** `transform`/renderers take a
   `&Palette`. `Mono`/`Inv` are computed against the resolved theme
   colour, so they remain correct per theme.
4. **Web output:** decide between
   (a) inline styles resolved server-side per theme (simple, but board
   HTML must be re-rendered on theme switch and per-user for shared
   renders), or
   (b) semantic CSS classes (`mk-fg-red`, `mk-bg-player-2`, modifier
   classes for mono/inv) with per-theme CSS custom properties - theme
   switch is instant and rendered HTML is theme-agnostic/cacheable.
   **Recommendation: (b)** - render output is pushed over WS and cached;
   theme-agnostic HTML avoids re-render churn. Mono/inv variants get
   precomputed custom properties per theme (`--mk-red`, `--mk-red-mono`,
   `--mk-red-inv`).
5. **Email/ANSI output:** these stay inline/concrete - they resolve
   against the user's theme `Palette` at render time (email) or map to
   the nearest ANSI colours as today.

## Theme set

Initial (proves the architecture): Neutral Light, Neutral Dark
(Material-derived), Dracula, Monokai.

Follow-up candidates - research task to confirm the current most popular
programmer themes, but the expected list based on editor-marketplace
popularity: One Dark (Atom), Solarized Light + Dark, Nord, Gruvbox
Light + Dark, Catppuccin (Mocha at minimum), Tokyo Night, GitHub
Light + Dark, Ayu, Everforest, Rosé Pine. All of these publish official
palettes with ANSI-16 mappings, which map naturally onto the named
palette (red/green/blue/etc. analogues exist in each).

Per-theme work is: fill in the `Palette` (21 named colours, player
sequence, surfaces/text), then validate contrast - especially fg-on-bg
pairs that games actually produce, and white/black text on each
player colour. A small contrast-check test (WCAG ratio over all
palette pairs the games use) should gate adding a theme.

## Open questions

- Go games: confirm what colour forms `brdgme-go` markup actually emits
  (named vs rgb) - determines whether the Go side needs any change.
- Whether "Auto" maps only to Neutral Light/Dark or lets the user pick a
  separate preferred light theme and dark theme (editor-style). Start
  with Auto = neutral pair; revisit.
- Whether Dracula should become the dark default once visible in situ
  (explicitly deferred).

## Dependencies / sequencing

- Post-go-live, non-blocking. Independent of other backlog items except:
  - #22b-d email work should land first (email templates to theme).
  - #25 rules rendering emails should consume the same themed email
    renderer.
- The markup semantic-colour change touches `rust/lib/color`,
  `rust/lib/markup`, all Rust games' markup emission, and render
  pipelines - it is the bulk of the risk and should be a standalone PR
  with unchanged-visual-output verification against the LIGHT palette
  (golden-file comparison of current vs new HTML output).
