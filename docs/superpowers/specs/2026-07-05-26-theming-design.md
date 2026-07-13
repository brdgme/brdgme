# 26: Theming / Dark Mode (Web UI + Email) - Design

> Extracted 2026-07-08 from `docs/plan/26-theming.md` (superpowers layout
> migration). Revised 2026-07-11 after the full colour audit: the palette
> is now the 14-slot contract in `docs/authoring/THEMING.md`, which is the
> authoritative palette reference - this spec covers architecture and
> sequencing and defers to it for slots, values, transforms, and contrast
> requirements.

**Status:** In progress - implementation plan at
`docs/superpowers/plans/2026-07-13-26-theming-semantic-colors.md`
**Added:** 2026-07-05
**Revised:** 2026-07-13 (`soften` transform replaces the two
BACKGROUND_SHADE slots; palette is now 12 slots - see THEMING.md)

## Decisions (2026-07-05)

- **Board colours: full remap per theme.** Each theme provides values for
  the full abstract palette: 12 slots (9 hues + GREY + FOREGROUND +
  BACKGROUND) plus the 8-colour player sequence - see
  `docs/authoring/THEMING.md`. No fixed-board shortcut.
- **`soften(color, pct)` transform (2026-07-13).** Surface steps and
  muted hue washes are derived, not slots: resolve the slot through the
  theme, then move HSL lightness toward BACKGROUND's lightness by pct%
  (hue/saturation kept). This one transform reproduces the old
  checkerboard shade slots (`soften(FOREGROUND, 86)`/`soften(FOREGROUND,
  75)` = `#dbdbdb`/`#bfbfbf` on light, vs the historical `220`/`190`
  greys) and Acquire's available-tile pink (`soften(PINK, 75)` =
  `#f7bed4` vs the historical Material Pink 100 `#f8bbd0`), and is
  direction-neutral on dark themes. Like `contrast`, it is evaluated
  after theme resolution, so markup stays fully abstract. Softened
  colours are backgrounds only.
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

1. **`brdgme-color`:** introduce a `NamedColor` enum covering the 12
   palette slots defined in `docs/authoring/THEMING.md` (RED ... BROWN,
   GREY, FOREGROUND, BACKGROUND) and a `Palette` type: `NamedColor ->
   Color` plus the 8-entry player-colour list, plus the `soften` and
   `contrast` resolution helpers. The
   current Material values become the built-in `LIGHT` palette
   (FOREGROUND/BACKGROUND replace BLACK/WHITE; the 8 unused constants
   are deleted and AMBER is renamed ORANGE per THEMING.md Appendix A).
2. **`brdgme-markup`:** replace `ColType::RGB` with
   `ColType::Named(NamedColor)` - concrete RGB is REMOVED from the
   public AST, not deprecated. Delete `From<brdgme_color::Color> for
   Col` so games cannot express a concrete colour at all; the rule is
   enforced by the type system. The parser maps legacy names via the
   aliases in THEMING.md (magenta -> PURPLE, amber -> ORANGE,
   black -> FOREGROUND, white -> BACKGROUND). Migrate the Rust games,
   including the two with bespoke RGB (acquire-1, lords-of-vegas-1 -
   exact remaps in THEMING.md Appendix A).
   **Go games (answered 2026-07-11):** `brdgme-go`'s `render.Fg`/`Bg`
   emit concrete `{{fg rgb(r,g,b)}}` triples even for named colours
   (`brdgme-go/render/color.go:43`), so Go output does NOT arrive named.
   `rgb(...)` therefore stays in the *grammar* as parse-time legacy
   compatibility only: the parser reverse-maps known triples (the 21
   old constants plus the 11 bespoke acquire-1/lords-of-vegas-1 values)
   to `Named` immediately, and unknown triples fall back to FOREGROUND
   with a warning - nothing concrete survives into the AST. This themes
   Go game output *and* historical markup already stored in the DB
   without touching Go. (`{{fg player(n)}}` and the `inv`/`mono` flags
   are already abstract in the grammar.) Emitting names from Go
   directly is optional follow-up, not required.
   `bot/system_prompt.md` documents the `rgb(r,g,b)` syntax to the LLM
   and must be updated to the named syntax so the bot stops emitting
   triples.
3. **Render-time palette resolution.** `transform`/renderers take a
   `&Palette`. The `contrast` transform (which replaces `Mono`/`Inv` -
   every real use was the composed readable-text idiom; see THEMING.md)
   is computed against the resolved theme colour, so it remains correct
   per theme. The parser still accepts `inv`/`mono` flags in legacy
   markup and normalises the composed pair to `contrast`.
4. **Web output:** decide between
   (a) inline styles resolved server-side per theme (simple, but board
   HTML must be re-rendered on theme switch and per-user for shared
   renders), or
   (b) semantic CSS classes (`mk-fg-red`, `mk-bg-player-2`, a modifier
   class for contrast) with per-theme CSS custom properties - theme
   switch is instant and rendered HTML is theme-agnostic/cacheable.
   **Recommendation: (b)** - render output is pushed over WS and cached;
   theme-agnostic HTML avoids re-render churn. Contrast variants get
   precomputed custom properties per theme (`--mk-red`,
   `--mk-red-contrast`).
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

Per-theme work is: fill in the `Palette` (14 slots + 8 player colours),
then validate the contrast requirements in `docs/authoring/THEMING.md` -
text colours (GREY especially) on BACKGROUND and both shades, `contrast`
output on every hue, player-colour distinguishability, and the specific
hue pairs games rely on. A small contrast-check test (WCAG ratio over
those pairs) should gate adding a theme. Themes typically cover most
slots natively and derive 1-2 (e.g. Dracula lacks blue and brown - see
the worked example in THEMING.md).

## Open questions

- ~~Go games: confirm what colour forms `brdgme-go` markup actually
  emits~~ Answered 2026-07-11: concrete `rgb(r,g,b)` triples; handled by
  parser reverse-mapping of the known legacy values (see step 2 above).
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
  `rust/lib/markup`, all Rust games' markup emission (including the
  bespoke-colour remaps in acquire-1 and lords-of-vegas-1 and the
  player-palette change - THEMING.md Appendix A), the
  `api/src/db/color.rs` player-preference enum (data migration:
  Amber -> Orange, BlueGrey removed, Cyan/Pink added), and render
  pipelines - it is the bulk of the risk and should be a standalone PR
  with unchanged-visual-output verification against the LIGHT palette
  (golden-file comparison of current vs new HTML output). Note the
  bespoke remaps (e.g. corp BLACK -> FOREGROUND, DEEP_ORANGE -> ORANGE)
  intentionally change output on the LIGHT palette in small ways;
  golden files for those two games are re-baselined with eyes on the
  diff rather than required to be byte-identical.
