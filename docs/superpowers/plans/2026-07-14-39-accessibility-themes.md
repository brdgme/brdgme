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

**Operator decision (2026-07-14, supersedes the category model above):**
Accessibility themes are categorised by colour blindness type instead of
one flat "Accessibility" category, because there will be multiple theme
choices per type. Combined-category model: `ThemeCategory::Accessibility`
is replaced by two categories - one displayed as "Deuteranopia /
Protanopia" and one displayed as "Tritanopia". Deutan- and
protan-targeted themes (near-identical palettes in practice) share the
combined category; tritan themes get their own. Default and Custom are
unchanged. Picker category order: Default (no heading), then
Deuteranopia / Protanopia, then Tritanopia, then Custom; alphabetical
sort within each group stays. WP3 adoptions go into the appropriate
CVD-type category rather than "Accessibility".

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
- [ ] WP2b: re-categorisation per operator decision - replace
  `ThemeCategory::Accessibility` with per-CVD-type categories
  ("Deuteranopia / Protanopia" combined, "Tritanopia"); re-tag the 6 WP2
  themes; update `grouped_themes()`/picker ordering, headings, unit
  tests; update THEMING.md category docs.
- [ ] WP3: third-party colourblind-first theme evaluation/adoption
  (adopt into the appropriate CVD-type category)
- [ ] Final verification (tests, checks, clippy, fmt)

## Handover (paused 2026-07-14 after WP1)

State: WP1 complete, verified, and committed. WP2/WP3 not started.

What exists now:
- `brdgme_color::ThemeCategory { Default, Accessibility, Custom }`;
  `themes()` returns `&[(&str, ThemeCategory, &Palette)]`. brdgme
  light/dark = Default, the other 24 = Custom. No Accessibility themes
  registered yet.
- Ordering contract (decided in WP1): `themes()` registry order stays
  authoritative and `THEME_SLUGS` mirrors it exactly (the
  `theme_slugs_match_brdgme_color_themes` test still asserts order).
  Grouping + alphabetical sorting happen only in the picker via the pure
  `grouped_themes()` in `rust/web/src/theme.rs` (unit-tested).
- Picker (`ThemeSettingsPage`, `rust/web/src/app.rs`): System tile first,
  Default tiles with no heading, then "Accessibility"/"Custom" `<h2
  class="theme-category-heading">` sections (styled in `main.scss` with
  flex-basis 100% inside the flex `.theme-grid`). Empty categories are
  omitted, so the Accessibility heading appears automatically once WP2
  registers themes.

Next steps (WP2, then WP3 - full specs are in the orchestrator brief /
Goal sections above):
- WP2: add 6 Accessibility themes ("brdgme light/dark
  deuteranopia/protanopia/tritanopia") to palette.rs keeping base
  neutrals, hues derived from Okabe-Ito / Paul Tol; implement
  Viénot/Brettel-style deutan/protan/tritan simulation matrices in the
  palette.rs test module and add a `gate_cvd_simulation` test (keyed off
  CVD keywords in theme names) asserting pairwise player+GREY+FOREGROUND
  deltaE under simulation >= a calibrated floor (document achieved
  minima); append slugs to `THEME_SLUGS` (theme.rs) and the
  `THEME_BOOT_SCRIPT` array (app.rs) in registry order.
- WP3: evaluate GitHub Light/Dark Colorblind (primer/github-vscode-theme)
  and Modus Operandi/Vivendi Deuteranopia + Tritanopia
  (protesilaos/modus-themes) against upstream sources; map to 12 slots,
  pass contrast gate + matching CVD gate, adopt as Accessibility or
  record skip reasons here.
- Final verification commands are in **Verify** above; run per-WP and at
  the end. Update this log per WP.
