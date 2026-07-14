# Mix Transform Recovery Handover

**Status:** Implementation stopped. The `mix-transform` worktree is prunable,
and its uncommitted code changes are lost.

**Purpose:** Recreate the completed Task 1 work and finish Tasks 2 and 3
without relying on the lost worktree or prior chat history.

## Surviving Records

- Design: `docs/superpowers/specs/2026-07-14-mix-transform-design.md`
- Original implementation plan: `docs/superpowers/plans/2026-07-14-mix-transform.md`
- Initial Task 1 report: `.superpowers/sdd/task-1-report.md`
- Corrected Task 1 report: `.superpowers/sdd/task-1-repair-report.md`
- Lost branch base: `4b77b6a1123973240e7edac56444f02347c7f930`
- Lost branch name: `mix-transform`

The design and original plan are untracked in the shared checkout. Preserve
them. No code changes from this work were committed or staged.

## Goal

Replace HSL-lightness `soften` behavior with sRGB channel interpolation and
add an explicit `mix(source, target, pct)` markup transform. Existing
`soften(color, pct)` remains valid and becomes
`mix(color, background, pct)`.

## Binding Decisions

- Mix every sRGB channel independently with half-up rounding.
- `pct = 0` returns `source`; `pct = 100` returns `target`.
- Markup accepts percentages only in `0..=100`, for both `soften` and `mix`.
- Retain existing `soften(...)` syntax, `Named { soften: ... }` AST form, and
  `soften-*` class-name scheme for stored markup compatibility.
- Add `mix(source, target, pct)` alongside `soften` for palette-slot mixes.
- Keep accessibility floors exactly at `3.0` for text and `4.5` for contrast
  transforms. Do not lower either floor.
- Do not modify palette definitions, CVD theme definitions, theme categories,
  or picker code.
- Do not add dependencies or a database migration.
- Do not edit the historical
  `docs/superpowers/plans/2026-07-13-26-theming-semantic-colors.md`.
- Do not commit unless explicitly requested.

## Approved Surface Migration

**Superseded 2026-07-15:** the user decided games and web chrome standardize
on `soften(80)` and `soften(90)` only — `soften(75)` is dropped everywhere,
not just for the foreground checkerboard, because sRGB `soften(_, 75)` no
longer clears the 3:1 contrast floor on every registered theme (Solarized
Dark's grey against `soften(foreground, 75)` measured 2.86:1). This also
moves Acquire's pink wash from 75 to 80. `IN_USE_SOFTENS` must contain
exactly:

```rust
pub const IN_USE_SOFTENS: &[(NamedColor, u8)] = &[
    (NamedColor::Foreground, 90),
    (NamedColor::Foreground, 80),
    (NamedColor::Pink, 80),
];
```

- Acquire checkerboard: foreground `86/75` becomes `90/80`.
- Acquire pink wash: `75` becomes `80`.
- Lords of Vegas unbuilt tile: foreground `78` becomes `80`.
- Shared sidebar/header/chrome surface: foreground `90`.
- Orange/red state cues remain `86`.
- Foreground hover remains `96`.
- Update every shared SCSS reference from
  `--mk-soften-foreground-86` to `--mk-soften-foreground-90`.

## Required Exact Values

| Expression | Expected value |
| --- | --- |
| `mix(DRACULA.foreground, DRACULA.background, 86)` | `#454751` |
| `mix(DRACULA.foreground, DRACULA.background, 90)` | `#3d3f49` |
| `soften(LIGHT.foreground, 90, LIGHT.background)` | `#e6e6e6` |
| `soften(LIGHT.foreground, 80, LIGHT.background)` | `#cccccc` |
| `soften(LIGHT.pink, 75, LIGHT.background)` | `#f0c5d6` (historical; 75 no longer shipped) |
| `soften(LIGHT.pink, 80, LIGHT.background)` | `#f3d1de` |
| `mix(LIGHT.red, LIGHT.blue, 50)` | `#765381` |

`#454549` is incorrect and must not appear as the expected 86% Dracula sRGB
result.

## Task 1: Recreate The Lost Resolver And CSS Work

### Files

- `rust/lib/color/src/palette.rs`
- `rust/lib/color/src/css.rs`
- `rust/lib/color/src/lib.rs`
- `rust/lib/markup/src/html_class.rs`
- `rust/web/src/theme.rs`

### Resolver

Add this public resolver in `palette.rs` and make `soften` delegate to it:

```rust
/// Mixes `source` toward `target` in sRGB by `pct` percent.
pub fn mix(source: Color, target: Color, pct: u8) -> Color {
    let weight = f64::from(pct.min(100)) / 100.0;
    let channel = |source: u8, target: u8| {
        round_u8(f64::from(source) + (f64::from(target) - f64::from(source)) * weight)
    };
    Color {
        r: channel(source.r, target.r),
        g: channel(source.g, target.g),
        b: channel(source.b, target.b),
    }
}

/// Derives a surface by mixing `color` toward `background` in sRGB.
pub fn soften(color: Color, pct: u8, background: Color) -> Color {
    mix(color, background, pct)
}
```

Remove the now-unused HSL conversion helpers (`rgb_to_hsl`, `hue_to_rgb`, and
`hsl_to_rgb`) and stale comments that refer to them. Their removal is required
to keep test output warning-free.

### Palette Tests And Contrast Gate

- Add exactness coverage for 0%, 100%, Dracula 86%, Dracula 90%, the light
  90/80 checkerboard, and the light pink 80 wash.
- Assert `soften(DRACULA.foreground, 86, DRACULA.background) ==
  mix(DRACULA.foreground, DRACULA.background, 86)`.
- Restore these constants exactly:

```rust
const TEXT_FLOOR: f64 = 3.0;
const TRANSFORM_FLOOR: f64 = 4.5;
```

- Extend the existing derived-surface list in the contrast gate with every
  `IN_USE_MIXES` entry, resolved by `mix(palette.color(source),
  palette.color(target), pct)` and named `mix(source, target, pct)`.

### CSS Registry And Custom Properties

In `rust/lib/color/src/css.rs`:

```rust
pub type MixExpression = (NamedColor, NamedColor, u8);

pub const IN_USE_MIXES: &[MixExpression] = &[];
```

Change the API to:

```rust
pub fn palette_css_vars(
    palette: &Palette,
    soften_exprs: &[(NamedColor, u8)],
    mix_exprs: &[MixExpression],
) -> String
```

For every mix expression, emit both:

```text
--mk-mix-<source>-<target>-<pct>: <mixed-color>;
--mk-mix-<source>-<target>-<pct>-contrast: <palette foreground-or-background>;
```

Use:

```rust
let mixed = mix(palette.color(source), palette.color(target), pct);
let token = format!("mix-{source}-{target}-{pct}");
```

Add a CSS test that `mix(red, blue, 50)` emits
`--mk-mix-red-blue-50: #765381;` and its contrast property.

### Exports And Existing Callers

- Re-export `mix`, `MixExpression`, and `IN_USE_MIXES` from
  `rust/lib/color/src/lib.rs`.
- Update all color-crate test callers of `palette_css_vars` with a third
  `&[]` argument.
- Update `rust/web/src/theme.rs` to import `IN_USE_MIXES` and pass it as the
  third argument in all three `palette_css_vars` calls: root light theme,
  system dark media query, and each named theme block.
- `rust/lib/markup/src/html_class.rs` has an existing CSS test that hardcodes
  `foreground-86`; change that test expectation to `foreground-90` because it
  verifies the static `IN_USE_SOFTENS` registry.

### Task 1 Evidence Previously Obtained

- RED: restoring floors before updating `IN_USE_SOFTENS` made the contrast
  gate fail for Solarized Dark at foreground 75:

```text
[solarized dark] grey vs soften(foreground, 75): 2.86 < 3
```

- GREEN: `cargo test -p brdgme_color` passed 16 tests.
- `cargo test -p brdgme_markup` passed 36 tests after the HTML-class test
  expectation changed to foreground 90.
- `cargo fmt --check -p brdgme_color -p brdgme_markup` passed.
- Task 1 received a spec-and-quality review with no Critical or Important
  findings.

## Task 2: Explicit Mixes In Markup And Semantic CSS

### Files

- `rust/lib/markup/src/ast.rs`
- `rust/lib/markup/src/parser.rs`
- `rust/lib/markup/src/transform.rs`
- `rust/lib/markup/src/semantic.rs`
- `rust/lib/markup/src/html_class.rs`

### AST And Serialization

Add this variant without changing `Named { color, soften }`:

```rust
Mix {
    source: NamedColor,
    target: NamedColor,
    pct: u8,
},
```

Serialize it as:

```rust
format!("mix({}, {}, {})", source, target, pct)
```

Add an AST round-trip test for `mix(red, blue, 50)`.

### Parsing

- Add a shared percentage parser that accepts only `0..=100`.
- Use it for both `soften` and `mix`.
- Add `col_type_mix` before `col_type_soften` so the `mix(` prefix is selected
  first.
- Resolve both named arguments with the existing `resolve_named` function.
- Accept both `mix(red, blue, 50)` and `mix(red,blue,50)`.
- Reject `mix(red,blue,101)` and out-of-range soften inputs without producing
  a color node.

### Concrete And Semantic Rendering

- In `transform.rs`, resolve `ColType::Mix` through `brdgme_color::mix` using
  palette-resolved source and target colors.
- Add the matching `SemanticColType::Mix` form and map to it in the semantic
  transform.
- Test concrete `Mix { Red, Blue, 50 }` against `LIGHT` as `#765381`.
- Test the semantic value has `source: Red`, `target: Blue`, and `pct: 50`.

### HTML Classes

- Format explicit mix classes as `mix-<source>-<target>-<pct>`.
- Ensure rendered HTML contains:

```html
<span class="mk-bg-mix-red-blue-50">x</span>
```

- Extend `markup_class_css` to generate structural foreground/background and
  contrast rules for every `IN_USE_MIXES` entry, referencing the corresponding
  `--mk-mix-*` variables.

### Task 2 Verification

Run from `rust/`:

```bash
cargo test -p brdgme_markup mix_parsing_works
cargo test -p brdgme_markup transform_with_palette_mix_works
cargo test -p brdgme_markup mix_works
cargo test -p brdgme_markup
cargo fmt --check -p brdgme_markup
```

Use TDD: write and observe the focused tests fail before production changes,
then run the complete markup suite after implementation.

## Task 3: Theme Integration, Chrome Migration, And Documentation

### Files

- `rust/web/src/theme.rs`
- `rust/web/style/main.scss`
- `docs/authoring/THEMING.md`

### Required Web Changes

- Keep the Task 1 `IN_USE_MIXES` wiring in all generated palette blocks.
- Add a `theme.rs` test that extracts the Dracula block from `THEME_STYLE_CSS`
  and asserts:

```rust
assert!(dracula.contains("--mk-soften-foreground-90: #3d3f49;"));
```

- Replace every generic shared foreground surface reference in `main.scss`:

```text
--mk-soften-foreground-86
```

with:

```text
--mk-soften-foreground-90
```

This covers sidebar/menu/header/meta surfaces, shared borders, recent-log
headers, suggestions, disabled inputs, current-turn chrome, and theme-tile
borders. Do not alter orange/red 86 state-cue references or foreground 96
hover references.

### Required Authoring Documentation Changes

Update `docs/authoring/THEMING.md` to state:

- `mix(source, target, pct)` linearly interpolates sRGB channels.
- Its endpoints are source at 0 and target at 100.
- `soften(color, pct)` means `mix(color, BACKGROUND, pct)`.
- Dracula surfaces become background-tinted rather than neutral grey.
- The shared foreground-surface migration is 90/80 where specified above.
- The brdgme light checkerboard is now `#e6e6e6`/`#cccccc`.
- Acquire `soften(pink, 80)` (moved from 75, dropped along with the
  foreground 75 stop) changes from the old HSL result `#f7bed4` to
  `#f3d1de`.
- The 2026-07-13 historical plan remains unchanged; the 2026-07-14 design is
  its superseding record.

### Task 3 Verification

Run from `rust/`:

```bash
SQLX_OFFLINE=true cargo test -p web --lib --features ssr theme::tests
cargo test -p brdgme_color
cargo test -p brdgme_markup
SQLX_OFFLINE=true cargo check -p web --features ssr
cargo fmt --check
```

If the development site is running, manually verify:

- Dracula sidebar menu, game metadata panel, and narrow header are blue-grey
  and visually close to `#282a36`.
- brdgme light Acquire checkerboard is `#e6e6e6`/`#cccccc`.
- Switching themes updates the surfaces without a reload.

## Execution And Review Procedure

- Create a new isolated worktree before recreating code. The previous path is
  unavailable and listed as prunable by `git worktree list`.
- Start from the intended current branch state, not from the prunable worktree.
- Execute Task 1 first because Tasks 2 and 3 consume its public APIs.
- Use TDD for each task, including a real RED run before implementation.
- Use Subagent-Driven Development as requested: fresh implementer, task-scoped
  spec-and-quality review, fixes and re-review for any Critical or Important
  finding, then proceed.
- Perform a whole-branch review after Task 3.
- Do not treat Task 1's past test report as proof for the reconstructed code;
  rerun its tests after recreation.

## Known Non-Blocking Review Notes

The previous Task 1 review recorded two Minor items only:

- The pink exact-value assertion appeared in both `soften_exactness` and
  `mix_exactness`. Fix alongside the 75→80 pink migration: keep one
  authoritative assertion at `soften(LIGHT.pink, 80, LIGHT.background) ==
  "#f3d1de"` and remove the duplicate.
- A generic CSS-variable test used foreground 86 as non-production input.

Neither item blocked Task 1. They may be cleaned up only if doing so does not
obscure the required regression coverage or expand scope.
