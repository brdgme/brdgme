# 39: Accessibility Themes + Theme Picker Categories Implementation Plan

**Created:** 2026-07-14
**Backlog:** #39 (top priority). Builds on #26's theme system
(`brdgme_color::themes()` registry, contrast gate, web picker at /theme,
`THEME_SLUGS`/`THEME_BOOT_SCRIPT` wiring; see
`2026-07-13-26-theming-semantic-colors.md` and
`2026-07-13-26-popular-themes.md`).

**Goal:**
1. CVD (colour vision deficiency) variants of the two default themes:
   brdgme light and brdgme dark each get deuteranopia, protanopia, and
   tritanopia variants (6 new themes), derived from established CVD-safe
   palettes (Okabe-Ito / Paul Tol) adapted to the light/dark base neutrals,
   validated under CVD simulation (the pairwise-distinguishability gate
   checks re-run with simulated CVD transforms) plus the normal contrast
   gate.
2. Theme categories: each registered theme gains a category -
   Default (brdgme light/dark), Accessibility (CVD variants + adopted
   third-party accessibility themes), Custom (all the rest). The /theme
   picker renders grouped: Default first with no heading, then
   "Accessibility", then "Custom"; themes sorted alphabetically by display
   name within each category (System tile stays first overall).
3. Evaluate + adopt third-party colourblind-first themes if high quality:
   candidates are GitHub Dark/Light Colorblind (official
   primer/github-vscode-theme variants) and Modus deuteranopia/tritanopia
   variants (protesilaos/modus-themes). Verify palettes against upstream
   (the usual rule: do not trust secondhand tables); map to the 12 slots,
   derive gaps, pass the contrast gate. Adopt into the Accessibility
   category; skip (and record why) if a candidate cannot meet the gates
   without losing its identity.

**Operator decision (2026-07-14, first revision, superseded by the next
paragraph):** Accessibility themes are categorised by colour blindness
type instead of one flat "Accessibility" category, because there will be
multiple theme choices per type. Combined-category model:
`ThemeCategory::Accessibility` is replaced by two categories - one
displayed as "Deuteranopia / Protanopia" and one displayed as
"Tritanopia". Deutan- and protan-targeted themes (near-identical palettes
in practice) share the combined category; tritan themes get their own.

**Operator decision (2026-07-14, final, supersedes both paragraphs
above):** the flat "Custom" category is also split by background
lightness, giving five categories total. `ThemeCategory` variants (exact
Rust identifiers left to the implementer, semantics fixed): `Default`,
`Light`, `Dark`, `DeutanProtan`, `Tritan`. Display strings: "Light",
"Dark", "Deuteranopia / Protanopia", "Tritanopia" (`Default` still
renders with no heading). Each theme keeps exactly one `ThemeCategory` -
no multi-category/duplicate-tile membership - because in this theme set
deutan and protan always travel together on one shared palette, so the
overlap between the two CVD types is total, not partial; a single
`DeutanProtan` category loses no information the scheme would otherwise
need to represent. Picker order (top to bottom):
1. No heading: System tile, brdgme light, brdgme dark (`Default`,
   unchanged).
2. "Light" heading: non-default, non-CVD themes with light backgrounds -
   alphabetical.
3. "Dark" heading: non-default, non-CVD themes with dark backgrounds -
   alphabetical.
4. "Deuteranopia / Protanopia" heading: CVD themes covering deutan and/or
   protan - alphabetical.
5. "Tritanopia" heading: CVD themes for tritanopia - alphabetical.

Existing themes are re-tagged, not moved: the 24 former-`Custom` themes
split into `Light`/`Dark` by their actual background colour (read each
palette's background slot, don't guess from the name alone); the 6 CVD
themes keep the deutan/protan-vs-tritan split from the first revision
above (4 -> `DeutanProtan`, 2 -> `Tritan`).

**Verify:** cargo test -p brdgme_color (contrast gate incl. any new CVD
gate); SQLX_OFFLINE=true cargo check -p web --features ssr / --features
hydrate; SQLX_OFFLINE=true cargo test -p web --lib --features ssr (theme
sync tests; sqlx DB failures expected); clippy -D warnings
(brdgme_color, web ssr); cargo fmt --check.

## Progress log

- [x] Orchestrator dispatched
- [x] WP1: theme categories infrastructure + grouped picker.
  `themes()` now returns `(&str, ThemeCategory, &Palette)`; `ThemeCategory
  { Default, Accessibility, Custom }` exported from brdgme_color. Contract:
  registry/`THEME_SLUGS` keep registry order (sync test unchanged in
  spirit); grouping + alphabetical sort live only in the picker, via the
  pure `grouped_themes()` in web/src/theme.rs (unit-tested: category
  order, alpha sort per group, total count, defaults membership). Picker
  renders System tile first, Default tiles with no heading, then
  Accessibility/Custom headings (`.theme-category-heading`, flex-basis
  100% inside the flex `.theme-grid`). Verified: brdgme_color tests, web
  theme tests, checks (ssr+hydrate), clippy, fmt.
- [x] WP2: CVD variants of brdgme light/dark (6 themes) + CVD simulation
  gate. 6 Accessibility themes registered: brdgme light/dark x
  deuteranopia/protanopia/tritanopia (deutan+protan share one palette per
  base, tuned to clear both simulations; Okabe-Ito-seeded hues, base
  neutrals kept). `gate_cvd_simulation` (palette.rs test module) keys off
  CVD keywords in theme names and asserts pairwise player+GREY+FOREGROUND
  deltaE under matching Vienot/Brettel-style simulation >=
  `CVD_DISTINCT_DELTA_E` (10.0). Note: the initial simulation paired the
  unnormalized Vienot 1999 HPE matrix with coefficients derived for a
  D65-normalized matrix (simulated white came out cyan); fixed in dd7d350
  to the matched Vischeck/daltonize constants, pinned by
  `cvd_simulation_preserves_achromatic` (white/grey/black are exact fixed
  points), with three palettes re-tuned under the corrected simulation.
  Achieved minima (independently re-verified by review): light
  deutan/protan 15.33 (blue/purple) / 15.43 (red/brown); dark
  deutan/protan 12.75 (green/grey) / 13.06 (cyan/pink); light tritan
  11.06 (brown/grey); dark tritan 11.58 (purple/brown) - all above the
  10.0 floor and the >=9 target. Slugs appended to `THEME_SLUGS`
  (theme.rs) and `THEME_BOOT_SCRIPT` (app.rs) in registry order.
  Verified: brdgme_color tests (13), web theme/sync tests, checks
  (ssr+hydrate), clippy (both, -D warnings), fmt. Review: spec compliant,
  clean; one deferred cosmetic note - DARK_DEUTERANOPIA's re-tuned PURPLE
  (hue 238.5) sits 6.2 degrees from its BLUE, reads blue-lavender, but
  both gates clear with margin.
- [ ] WP2b: re-categorisation per the final operator decision above (5
  categories: Default, Light, Dark, DeutanProtan, Tritan). A prior
  sub-orchestrator attempt died mid-flight (model usage limit, not a code
  problem) after implementing only the *first-revision* 4-category scheme
  (Default/DeutanProtan/Tritan/Custom, no Light/Dark split) as an
  uncommitted working-tree diff, plus 3 orphaned/unregistered draft Modus
  palette statics in `palette.rs` left over from a premature WP3 start.
  Nothing from that attempt was committed. WP2b redo picks up from
  scratch: 5-category enum, all 30 themes re-tagged (2 Default, 24
  Light/Dark split by actual background colour, 4 DeutanProtan, 2
  Tritan), `grouped_themes()` + picker + THEMING.md updated for the new
  order, orphaned Modus statics removed (or left for WP3 to redo
  verified, implementer's call - see WP3 note below).
- [ ] WP3: third-party colourblind-first theme evaluation/adoption
  (GitHub Light/Dark Colorblind + Modus Operandi/Vivendi
  Deuteranopia/Tritanopia; adopt into `DeutanProtan` or `Tritan`). Note:
  the dead attempt's scratchpad has unverified research drafts
  (`wp3-research-github.md`, `wp3-research-modus.md`, fetched upstream
  `.el`/`.json` files) - treat as unverified draft input only, re-fetch
  and re-verify every hex value against real upstream sources before
  trusting it.
- [ ] Final verification (tests, checks, clippy, fmt)

## Handover (updated 2026-07-14, second sub-orchestrator attempt)

State: WP1 and WP2 complete, verified, and committed (HEAD `d6632ed`).
WP2b and WP3 not started (a first sub-orchestrator attempt at WP2b/WP3
died mid-flight from a model usage limit; its uncommitted, superseded-scheme
draft work is discarded/redone rather than built on - see the WP2b/WP3
progress-log entries above for what it left behind).

What exists now (at `d6632ed`):
- `brdgme_color::ThemeCategory { Default, Accessibility, Custom }`;
  `themes()` returns `&[(&str, ThemeCategory, &Palette)]`. 2 Default
  (brdgme light/dark), 24 Custom, 6 Accessibility (the WP2 CVD variants).
- Ordering contract: `themes()` registry order stays authoritative and
  `THEME_SLUGS`/`THEME_BOOT_SCRIPT` mirror it exactly (the
  `theme_slugs_match_brdgme_color_themes` test asserts order). Grouping +
  alphabetical sorting happen only in the picker via the pure
  `grouped_themes()` in `rust/web/src/theme.rs` (unit-tested).
- Picker (`ThemeSettingsPage`, `rust/web/src/app.rs`): System tile first,
  Default tiles with no heading, then category headings (`<h2
  class="theme-category-heading">`, flex-basis 100% inside the flex
  `.theme-grid`). Empty categories are omitted.
- `gate_cvd_simulation` test (palette.rs) keyword-matches theme names for
  deuteranopia/protanopia/tritanopia; floor `CVD_DISTINCT_DELTA_E = 10.0`.

Next steps (WP2b, then WP3 - full specs are in the orchestrator brief /
Goal / operator-decision sections above): see the WP2b and WP3
progress-log bullets above for the concrete task breakdown and what to
watch out for from the dead attempt's leftovers. Final verification
commands are in **Verify** above; run per-WP and at the end. Update this
log per WP.
