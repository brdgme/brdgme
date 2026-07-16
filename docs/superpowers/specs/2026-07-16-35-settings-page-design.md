# 35: Profile/Settings Page - Design

**Status:** Decided 2026-07-16. Supersedes and extends the UI-relevant parts
of [2026-07-11-35-user-settings-design.md](2026-07-11-35-user-settings-design.md);
that spec's D2 (username rules), D3 (petname defaults + rename migration)
and D4 (colour prefs wired into game creation) remain in force. The
palette is now 8 colours: Green, Red, Blue, Orange, Purple, Brown, Cyan,
Pink (`rust/lib/color::player_colors()`); the older spec's 7-colour list
is stale. The `choose()` port landed 2026-07-16 in
`rust/web/src/db.rs::choose_colors`.

## Route and navigation

- New `/settings` route, `SettingsPage` component in a new
  `rust/web/src/settings.rs` module (app.rs is ~1000 lines; don't grow it),
  wrapped in `MainLayout`. Logged-in only.
- Sidebar link "Theme" (`components/layout.rs`) becomes "Settings" and
  points to `/settings`.
- The standalone `/theme` route and `ThemeSettingsPage` are deleted
  entirely (no redirect). Anonymous users keep whatever theme cookie they
  had (the boot script still honours it) but have no UI to change it.

## Page structure

Single column, constrained: `.settings { max-width: 40em; padding: 0 1em; }`
(also fixes the current theme page's missing left padding). Sections top
to bottom:

1. **Username** - text input prefilled with current name, pattern
   `[a-zA-Z0-9_-]{1,16}`, help text "1-16 characters: letters, numbers,
   - and _. Must be unique.", explicit Save button; server-side
   uniqueness failure shown as a field error ("That name is taken").
2. **Preferred colours** - exactly three native `<select>` boxes labelled
   1st/2nd/3rd choice, each listing the 8 palette colours. Default (and
   the interpretation of existing empty `pref_colors`) is palette order:
   Green, Red, Blue - behaviour-neutral since identical prefs resolve by
   rank with random tiebreak. Selecting a colour already chosen in
   another box swaps the two. Always valid, no empty state. Each select's
   current value is also shown as a colour chip (see swatch style) so the
   pick previews in the live theme. Saves immediately on change
   (fire-and-forget ServerAction, like the current theme tiles).
3. **Theme** - reworked picker, below. Applies immediately on click.
4. **Email addresses** - current login email rendered read-only plus
   muted "Additional email addresses are coming soon." Placeholder
   section; becomes a real list + add form (using the form template) when
   #22d lands.

Save model: per-section. Username has an explicit Save because it can
fail validation; colours and theme apply immediately. No page-wide
dirty-state.

## Theme picker rework

- Each category is its own block flowing down the page: `<h2>` heading,
  then a wrapping flex row of tiles. Removes the `flex-basis:100%`
  heading hack in `main.scss`.
- **Deuteranopia and Protanopia become separate categories**: split
  `ThemeCategory::DeutanProtan` into `Deutan` and `Protan` in
  `brdgme_color`; update the tests that enumerate categories.
- **Selected tile highlight**: same treatment as "active game / your
  turn" - `background-color: var(--mk-soften-orange-86); font-weight:700`
  on the tile's label bar via a `.selected` class. Selection state derives
  from the current theme signal, including the System tile.
- **Swatch redesign**: replace the current sample (Red/Blue/Grey words,
  player names, "Surface" chip, Bold) with one line of 8 colour chips:
  each chip is the colour name with a single space of padding either side,
  colour as background, contrast colour as text (` Green ` on green).
  No Bold, no player names. Implemented in `theme.rs` by changing
  `SAMPLE_MARKUP` to bg/contrast-fg markup for the 8 slots and dropping
  `sample_players()`/`sample_player_style()` and per-tile player vars.

## Reusable form template

CSS-only plus one small component, zero new dependencies. Intended as the
template for other important forms (e.g. new game) - migrating those is a
follow-up, not part of this work.

```
<div class="form-field">        // FormField(label, help, error, children)
  <label class="form-label">...</label>
  <div class="form-control">{input/select/chips}</div>
  <div class="form-help">...</div>    // optional, muted, 0.8em
  <div class="form-error">...</div>   // optional, var(--mk-red)
</div>
<div class="form-actions">{submit}{pending indicator}</div>
```

- `.form-label`: bold, `display:block` - label above control (collapses
  naturally on mobile, fits the monospace aesthetic).
- `.form-help`: `font-size:0.8em; color: var(--mk-grey)`.
- `.form-error`: `color: var(--mk-red)` (matches `.command-error`).
- `.form-field { margin-bottom: 1em }`.
- New `FormField` component in `components/`.

## Responsive

The 40em constraint plus existing breakpoints (sidebar collapse at 80em)
cover full width/tablet/mobile; tiles, chips and selects wrap; inputs get
`max-width:100%`. No new breakpoints.

## Components / server fns

- New: `settings.rs` (SettingsPage), `FormField` and `ColorChip` in
  `components/`, server fns `set_username`, `set_pref_colors` alongside
  the existing `set_theme`.
- `pref_colors` stores canonical colour names ("Green"...); display maps
  via `theme.rs::slot_from_color_name`. Legacy names ("Amber",
  "BlueGrey") are normalized on read (already handled in
  `db.rs::normalize_pref_color`).
- CSS: add `.settings`, `.form-*`, `.color-chip`, `.theme-category`,
  `.theme-tile .selected`; delete the theme heading flex hack and
  sample-player CSS.

## Non-goals

Drag-and-drop reordering, avatars, actual multi-email management (#22d),
notification toggles (#36), migrating existing forms to the template
(follow-up), anonymous theme UI.
