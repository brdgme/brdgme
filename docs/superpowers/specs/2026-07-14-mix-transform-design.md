# Mix Transform Design

**Status:** Approved 2026-07-14

## Problem

`soften(color, pct)` currently preserves `color`'s HSL hue and saturation and
only moves its lightness toward the theme background. A neutral foreground
therefore remains neutral on a coloured background. For example, Dracula's
`#f8f8f8` foreground produces the grey `#4b4b4b` for
`soften(foreground, 86)`, instead of a surface related to its `#282a36`
background. Acquire's checkerboard and web chrome surfaces consequently stand
out in themes with coloured backgrounds.

## Decision

Add a generic `mix(source, target, pct)` colour transform. It interpolates all
three sRGB channels, with `0` returning `source` and `100` returning `target`.

Retain `soften(color, pct)` as the convenient surface syntax. It resolves as
`mix(color, background, pct)`. Percentages are inclusive `0..=100`; the
current `1..=99` clamp is removed.

The transform applies after palette resolution, so games continue to emit only
named colours and markup remains theme-agnostic. The exact same Rust resolver
serves web, email, and ANSI output.

## Markup And Rendering

- Markup accepts `mix(source, target, pct)` alongside `soften(color, pct)`.
- `soften` resolves through the mix helper with `BACKGROUND` as its target,
  while retaining its existing representation and `soften-*` CSS-class tokens
  for stored-markup and chrome-variable compatibility.
- The web CSS generator emits custom properties for each mix expression used
  by games or chrome. Semantic board HTML references those properties, so
  client-side theme switching remains instant.
- `soften` remains the normal game-authoring form for a surface that should
  recede toward its theme background. `mix` is available where the target must
  be another palette slot.

## Web Chrome

Shared chrome uses the `--mk-soften-foreground-90` token, so its generated
value changes automatically:

- `.layout-header`, `.menu`, and `.game-meta` use the sidebar/header surface.
- `.game-main` borders, recent-log headers and borders, suggestions, disabled
  inputs, current-turn chrome, and theme-tile borders use the same surface.
- The existing orange/red/foreground chrome softens use the same mix resolver.

Under Dracula, `soften(foreground, 90)` changes from neutral `#4b4b4b` to
`#3d3f49`, a blue-grey surface close to its background. The former 86% sRGB
value is `#454751` (not `#454549`). The Acquire
checkerboard uses `soften(foreground, 90)` and `soften(foreground, 80)`;
Lords of Vegas' unbuilt tile uses `soften(foreground, 80)`. The stateful
orange/red chrome remains at 86 and foreground hover remains at 96.

## Compatibility And Documentation

- Existing stored markup containing `soften(...)` remains valid and gains the
  corrected theme-aware result when rendered.
- The light checkerboard moves to `#e6e6e6`/`#cccccc` as its foreground
  softens move from 86/75 to 90/80, placing both squares closer to the white
  background. Coloured washes intentionally change too: sRGB
  `soften(_, 75)` no longer clears the contrast floor on every registered
  theme (Solarized Dark's grey against `soften(foreground, 75)` measured
  2.86:1, below the 3:1 text floor), so Acquire's pink wash also moves from
  75 to 80. `soften(pink, 80)` becomes `#f3d1de` on brdgme light, rather than
  the former HSL-lightness 75% result `#f7bed4`.
- The historic HSL-lightness decision, D1 in
  `docs/superpowers/plans/2026-07-13-26-theming-semantic-colors.md`, is not
  edited. This document supersedes it.
- `docs/authoring/THEMING.md` is updated to make sRGB `mix` and the `soften`
  wrapper the authoritative palette contract.

## Verification

- Unit-test mix endpoints, midpoint, and `soften(color, pct) ==
  mix(color, background, pct)` for light and Dracula palettes.
- Update exact-value tests: brdgme light's checkerboard is
  `#e6e6e6`/`#cccccc`; brdgme light's pink wash and Dracula's derived surfaces
  use sRGB-mixed values.
- Extend parser, semantic transform, CSS-token, and palette-CSS tests for
  explicit mixes and canonicalised softens.
- Retain the contrast gate over every in-use derived surface, now including
  explicit mixes. Tune palettes only if the changed derived surface causes a
  contrast failure.
- Verify generated Dracula CSS gives the sidebars/header the background-tinted
  surface and run the existing web SSR/hydrate checks plus formatting and a
  quick manual theme-switch check.
