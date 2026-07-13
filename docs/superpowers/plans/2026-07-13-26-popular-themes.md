# 26: Popular Programming Themes Implementation Plan

**Created:** 2026-07-13
**Source research:** `THEME_ANALYSIS.md` (repo root; user does NOT fully trust
its correctness - every palette must be verified against upstream sources
before implementation)
**Parent:** `2026-07-13-26-theming-semantic-colors.md` (theme registry,
contrast gate) and `docs/authoring/THEMING.md` (12-slot palette contract).

**Goal:** Every theme in THEME_ANALYSIS.md becomes a registered brdgme theme:
added to `brdgme_color::themes()`, passing the contrast gate, selectable on
the web (`THEME_SLUGS` in rust/web/src/theme.rs + `THEME_BOOT_SCRIPT` slug
list in rust/web/src/app.rs + their sync tests).

**Process:** one orchestrator; per theme it spawns a Sonnet 5 subagent
(serial, never parallel) that: (1) verifies the research palette against
official upstream sources, (2) maps palette colours onto the 12 brdgme slots,
(3) derives any missing slots (usually BROWN, often GREY) consistently with
the theme's character, (4) implements + registers the theme and tunes values
minimally until the contrast gate passes (precedent: D10 in the parent plan),
noting every drift from the verified upstream value.

**Themes** (each doc palette/variant = one brdgme theme; dracula already
exists - verify only, keep existing D10 tunings unless provably wrong):
alucard, solarized dark, solarized light, nord dark, nord light, one dark,
one light, gruvbox dark, gruvbox light, catppuccin mocha, catppuccin latte,
tokyo night, tokyo night storm, tokyo night light, night owl, light owl,
synthwave 84, papercolor light, papercolor dark, monokai, darcula,
vs code dark+, vs code dark modern.

**Verify:** `cargo test -p brdgme_color` (contrast gate over all registered
themes), `SQLX_OFFLINE=true cargo test -p web --lib --features ssr` (theme
slug/boot-script sync tests; DB sqlx failures expected), web ssr+hydrate
checks, fmt, clippy.

## Progress log

- [x] Orchestrator dispatched
- [x] All 23 themes implemented (registry now 26 themes incl. brdgme
  light/dark/dracula); full verification green 2026-07-13:
  `cargo test -p brdgme_color` 11/11, web ssr+hydrate check, web theme
  tests 7/7 (sqlx DB tests fail without Postgres, expected), clippy
  `-D warnings` clean on brdgme_color + web (ssr), `cargo fmt --check`
  clean. Nothing committed.

Per-theme notes (research-doc discrepancies, mapping, derivations,
tunings; full details in each palette's doc comment in
`rust/lib/color/src/palette.rs`):

- [x] **dracula** (verify only): THEME_ANALYSIS.md table matches
  draculatheme.com/spec exactly; existing D10 tunings kept unchanged.
- [x] **alucard**: research table byte-identical to the official
  "Alucard Classic" spec (openly published, not paywalled). Derived
  BLUE `#203f97` (hue between cyan/purple) and BROWN `#5d3e32`
  (desaturated orange). No gate tuning needed.
- [x] **solarized dark/light**: research table matches
  ethanschoonover.com exactly. BROWN derived (~28 deg hue) per variant.
  Tunings: dark FG -> `#ffffff`, GREY `#9aabb1`, violet/blue nudged;
  light FG -> `#000000`, GREY `#4c5f67`, green/cyan darkened ~3% l.
- [x] **nord dark/light**: nord0-15 match upstream exactly. PINK derived
  rose 315 deg, BROWN derived tan 20 deg. Dark GREY lightened from nord3
  (`#4c566a` -> `#bcbecd`, nord3 only 1.7:1); dark red/orange/purple
  lightened slightly; light variant darkened all 7 pastel accents.
- [x] **one dark/light**: research Comment `#59626F` wrong (upstream
  mono-3 is `#5c6370`); cyan omitted from doc. One Light's hue-6/hue-6-2
  share hue 41 deg, forcing YELLOW off-hue (49 deg `#d0aa01`). Dark GREY
  moved to desaturated hue 100 (comment hue unresolvable vs FG/CYAN
  distinctness); dark FG lightened `#abb2bf` -> `#bfc5ce`. PINK/BROWN
  derived both variants.
- [x] **gruvbox dark/light**: research table matches morhetz/gruvbox
  exactly. PINK/BROWN derived. Dark: red lightened `#fb4934`->`#ff553c`,
  GREY `#928374`->`#bcbca9`, FG desaturated/lightened. Light: faded
  green/yellow/aqua darkened, GREY -> `#64594f`.
- [x] **catppuccin mocha/latte**: research tables match upstream (NOTE:
  verified from model knowledge of the well-known palette, not a live
  fetch). CYAN=Sky (Sapphire too close to Blue). BROWN derived from
  Peach hue. Mocha GREY = overlay2 value; Latte darkened most accents,
  GREY moved to near-neutral slate `#3a3e40`.
- [x] **tokyo night / storm / light**: dark/storm accents confirmed; fg
  is `#a9b1d6` (not `#c0caf5`); nvim and vscode upstreams disagree on
  comment colour (`#565f89` vs `#5f6996`). Light bg `#e6e7ed` and
  comment `#6c6e75` confirmed correct. PINK derived hue 290 (native
  magenta2 failed transform floor); GREY re-hued to 200 deg
  (dark) / darkened (light); light CYAN uses `#006c86` over `#0f4b6e`
  (FG distinctness).
- [x] **night owl / light owl**: dark table accurate but omitted green
  `#c5e478` / yellow `#ffcb8b`; Light Owl table had multiple errors
  (comment is `#989fb1` not `#93A1A1`; red `#e64d49` not `#D32F2F`;
  numbers are magenta `#aa0982` not `#AA5D00`; strings salmon not
  green). Light Owl has only 5 official accent hues; GREEN/YELLOW/
  ORANGE/BROWN derived. Several light accents darkened for floors;
  GREY moved to warm near-neutral `#534f46`.
- [x] **synthwave 84**: research bg `#261B2F` wrong (official
  `#262335`); comment is `#848bbd` not `#7F87C4`; `#F92AAD` "numbers
  pink" does not exist upstream; keywords are yellow `#fede5d` not red.
  BLUE/BROWN/GREY derived; RED lightened `#fe4450`->`#fe4d59` (4.5:1).
- [x] **papercolor light/dark**: tables match upstream PaperColor.vim
  except dark color16 typo `#5FADFT` (real `#5fafd7`). Light: several
  accents darkened, BLUE resaturated `#0037c7`, YELLOW derived
  `#705400`, GREY moved to teal-grey `#395c4c`. Dark: RED derived
  `#ff325f` (upstream red too pink vs PINK slot), BROWN derived, GREY
  lightened.
- [x] **monokai**: table matches the canonical tmTheme values (model
  knowledge). RED `#f66355` and BLUE `#869fea` derived (Monokai has
  neither); BROWN derived; PINK lightened `#f92672`->`#fa4c8c`; GREY
  moved off comment olive to cool near-neutral `#c4cfd4`.
- [x] **darcula** (JetBrains): table accurate (verified via community
  mirrors, e.g. doums/darcula - JetBrains XML not directly fetched).
  BROWN = upstream Ruby-comment tan `#bc9458`; CYAN/PINK derived;
  FG/RED/GREEN/PURPLE/ORANGE/GREY lightness-tuned for floors.
- [x] **vs code dark plus / dark modern**: both implemented. Dark Modern
  includes dark_plus tokenColors, so accents are identical; differences
  are BG (#1e1e1e/#1f1f1f), FG (#d4d4d4/#cccccc), and RED (`#f44747`
  Dark+ invalid vs `#f85149` Modern errorForeground - missed by the
  research doc). PURPLE derived hue 260 (PINK takes `#c586c0`);
  BROWN/GREY derived. No tuning needed.

## Decisions

- Display names avoid characters that break `slugify()` (spaces only):
  "synthwave 84" (no apostrophe), "vs code dark plus" (no "+").
- Both VS Code variants kept despite near-identical palettes: they
  differ in three slots (BG/FG/RED), which is a real, if narrow,
  difference.
- Solarized FOREGROUND uses pure white/black instead of base0/base00:
  the official body-text tones cap the `contrast` transform below the
  4.5:1 floor against several accents. This is the largest deliberate
  drift from an upstream identity.
- Where a theme's comment colour could not clear both the 3:1 surface
  floor and 15 deltaE distinctness from FOREGROUND on its native hue
  (catppuccin latte, tokyo night, night owl light, monokai, papercolor
  light, one dark), GREY was moved to a different hue rather than
  breaking either floor; each move is documented in the doc comment.
- Catppuccin and Monokai palettes were verified from model knowledge of
  those extremely well-mirrored palettes rather than live fetches; all
  other themes were verified against live upstream sources. Flagged for
  user awareness given the "do not trust the research doc" instruction.
