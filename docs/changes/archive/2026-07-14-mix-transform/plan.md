# Mix Transform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace HSL-lightness softening with an sRGB mix-to-background wrapper, and expose reusable explicit palette-slot mixes across every renderer.

**Architecture:** `brdgme_color::mix` is the single concrete sRGB resolver; `soften` delegates to it with the palette background as target. Markup gains an explicit `Mix` variant, while the existing `Named { soften: ... }` representation and `soften-*` CSS tokens remain intact so stored markup and web chrome continue to work unchanged structurally. Explicit mixes follow the existing in-use-expression registry pattern, allowing the semantic CSS renderer to emit stable, theme-switchable custom properties.

**Tech Stack:** Rust 2024, `brdgme_color`, `brdgme_markup`, Leptos web CSS custom properties, SCSS.

## Global Constraints

- Interpolate every `mix` and `soften` channel in sRGB, not HSL.
- Percentages are valid only from `0` through `100`, inclusive; `0` returns the source and `100` returns the target.
- `soften(color, pct)` remains supported and resolves as `mix(color, background, pct)`.
- Preserve the `soften-*` CSS variable and class naming scheme. Migrate the shared foreground surface references from 86 to 90; keep orange/red state cues at 86 and foreground hover at 96.
- Keep the historic HSL decision in `2026-07-13-26-theming-semantic-colors.md` unchanged; update only the authoritative `docs/authoring/THEMING.md` and the new mix design spec.
- Do not edit theme palettes, CVD theme definitions, categories, or picker code.
- Add no dependencies and do not create a database migration.
- Do not commit changes unless the user explicitly requests a commit.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `rust/lib/color/src/palette.rs` | sRGB `mix`, `soften` delegation, derived-surface contrast coverage. |
| `rust/lib/color/src/css.rs` | Registry and CSS custom-property emission for explicit mixes. |
| `rust/lib/color/src/lib.rs` | Public re-exports of mix APIs. |
| `rust/lib/markup/src/ast.rs` | Explicit symbolic `Mix` AST representation and markup serialization. |
| `rust/lib/markup/src/parser.rs` | `mix(...)` parsing and 0..=100 percentage validation. |
| `rust/lib/markup/src/transform.rs` | Concrete email/ANSI/plain mix resolution through `brdgme_color::mix`. |
| `rust/lib/markup/src/semantic.rs` | Theme-agnostic web semantic mix representation. |
| `rust/lib/markup/src/html_class.rs` | Stable `mix-<source>-<target>-<pct>` CSS class tokens and rules. |
| `rust/web/src/theme.rs` | Emit mix variables in each theme block and pin Dracula's shared chrome surface. |
| `docs/authoring/THEMING.md` | Authoritative mix/soften contract and revised Acquire pink example. |

### Task 1: Add The sRGB Resolver And CSS Variables

**Files:**
- Modify: `rust/lib/color/src/palette.rs:3125-3185,3260-3269`
- Modify: `rust/lib/color/src/css.rs:1-80`
- Modify: `rust/lib/color/src/lib.rs:8-13`

**Interfaces:**
- Produces: `pub fn mix(source: Color, target: Color, pct: u8) -> Color`.
- Produces: `pub fn soften(color: Color, pct: u8, background: Color) -> Color`, delegating to `mix`.
- Produces: `pub type MixExpression = (NamedColor, NamedColor, u8)` and `pub const IN_USE_MIXES: &[MixExpression]`.
- Produces: `pub fn palette_css_vars(palette: &Palette, soften_exprs: &[(NamedColor, u8)], mix_exprs: &[MixExpression]) -> String`.
- Consumed by Task 2: mix resolver and explicit-mix registry.
- Consumed by Task 3: three-argument `palette_css_vars` and `IN_USE_MIXES`.

- [ ] **Step 1: Write failing resolver and CSS-variable tests**

Add these tests to `palette.rs` before changing the resolver:

```rust
#[test]
fn mix_exactness() {
    assert_eq!(mix(DRACULA.foreground, DRACULA.background, 0), DRACULA.foreground);
    assert_eq!(mix(DRACULA.foreground, DRACULA.background, 100), DRACULA.background);
    assert_eq!(
        mix(DRACULA.foreground, DRACULA.background, 86).hex(),
        "#454751"
    );
    assert_eq!(
        mix(DRACULA.foreground, DRACULA.background, 90).hex(),
        "#3d3f49"
    );
    assert_eq!(soften(LIGHT.pink, 75, LIGHT.background).hex(), "#f0c5d6");
}

#[test]
fn soften_is_a_mix_to_background() {
    assert_eq!(
        soften(DRACULA.foreground, 86, DRACULA.background),
        mix(DRACULA.foreground, DRACULA.background, 86)
    );
}
```

Replace the former checkerboard assertions in `soften_exactness` with
`soften(LIGHT.foreground, 90, LIGHT.background) == #e6e6e6` and
`soften(LIGHT.foreground, 80, LIGHT.background) == #cccccc`; retain the pink
assertion as `#f0c5d6`. Add a CSS test in `css.rs`:

```rust
#[test]
fn palette_css_vars_contains_mix_and_contrast() {
    let css = palette_css_vars(
        &LIGHT,
        &[],
        &[(NamedColor::Red, NamedColor::Blue, 50)],
    );
    assert!(css.contains("--mk-mix-red-blue-50: #765381;"));
    assert!(css.contains("--mk-mix-red-blue-50-contrast:"));
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test -p brdgme_color mix_exactness`

Run: `cargo test -p brdgme_color palette_css_vars_contains_mix_and_contrast`

Expected: compilation failure because `mix`, the third `palette_css_vars` argument, and explicit mix CSS output do not exist.

- [ ] **Step 3: Implement sRGB mix and the explicit-mix CSS registry**

Replace the HSL-lightness body of `soften` with the shared resolver:

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

In `css.rs`, add the public registry, initially empty because no game currently emits an explicit `mix` expression:

```rust
pub type MixExpression = (NamedColor, NamedColor, u8);

pub const IN_USE_MIXES: &[MixExpression] = &[];
```

Extend `palette_css_vars` to take `mix_exprs`. For each `(source, target, pct)`, emit both `--mk-mix-{source}-{target}-{pct}` and its `-contrast` counterpart from:

```rust
let mixed = mix(palette.color(source), palette.color(target), pct);
let token = format!("mix-{source}-{target}-{pct}");
```

Keep the existing soften loop and its `soften-*` names exactly as-is. Re-export `mix`, `MixExpression`, and `IN_USE_MIXES` from `lib.rs`.

Update `in_use_surfaces` in `palette.rs` to append `IN_USE_MIXES`, resolving each entry with `mix`; this keeps the existing contrast gate exhaustive as games adopt explicit mixes.

- [ ] **Step 4: Run core tests and formatting**

Run: `cargo test -p brdgme_color`

Expected: all `brdgme_color` tests pass, including the contrast gate across all registered themes.

Run: `cargo fmt --check -p brdgme_color`

Expected: exits 0.

### Task 2: Carry Explicit Mixes Through Markup And Semantic CSS

**Files:**
- Modify: `rust/lib/markup/src/ast.rs:52-137`
- Modify: `rust/lib/markup/src/parser.rs:49-180,480-628`
- Modify: `rust/lib/markup/src/transform.rs:14-37,434-454`
- Modify: `rust/lib/markup/src/semantic.rs:14-40,104-143`
- Modify: `rust/lib/markup/src/html_class.rs:1-80,123-231`

**Interfaces:**
- Consumes: `brdgme_color::mix`, `IN_USE_MIXES`, and `MixExpression` from Task 1.
- Produces: `ColType::Mix { source: NamedColor, target: NamedColor, pct: u8 }`.
- Produces: `SemanticColType::Mix { source: NamedColor, target: NamedColor, pct: u8 }`.
- Produces: `mix(source, target, pct)` markup and `mix-<source>-<target>-<pct>` semantic CSS tokens.
- Consumed by Task 3: CSS rules and static theme custom properties for registered explicit mixes.

- [ ] **Step 1: Write failing AST, parser, concrete-renderer, and semantic-renderer tests**

Add a parser test that checks both spacing forms and the inclusive boundary:

```rust
#[test]
fn mix_parsing_works() {
    let (parsed, rest) = markup().parse("{{bg mix(red, blue, 50)}}x{{/bg}}").unwrap();
    assert_eq!(rest, "");
    assert_eq!(
        parsed,
        vec![N::Bg(
            Col {
                color: ColType::Mix {
                    source: NamedColor::Red,
                    target: NamedColor::Blue,
                    pct: 50,
                },
                transform: vec![],
            },
            vec![N::text("x")],
        )]
    );
    if let Ok((nodes, _)) = markup().parse("{{bg mix(red,blue,101)}}x{{/bg}}") {
        assert!(
            !nodes.iter().any(|node| matches!(node, N::Bg(..))),
            "out-of-range mix must not produce a background node: {nodes:?}"
        );
    }
}
```

Add a transform test that renders `Mix { Red, Blue, 50 }` against `LIGHT` and expects `Color::from_hex("#765381").unwrap()`. Add semantic and HTML-class tests that expect `SemanticColType::Mix { source: Red, target: Blue, pct: 50 }` and:

```rust
r#"<span class="mk-bg-mix-red-blue-50">x</span>"#
```

Extend the AST round-trip test with that same mix node and assert `to_string` emits `mix(red, blue, 50)`.

- [ ] **Step 2: Run markup tests to verify they fail**

Run: `cargo test -p brdgme_markup mix_parsing_works`

Run: `cargo test -p brdgme_markup transform_with_palette_mix_works`

Run: `cargo test -p brdgme_markup mix_works`

Expected: compilation failure because `ColType::Mix`, `SemanticColType::Mix`, and the mix parser/token paths do not exist.

- [ ] **Step 3: Add the explicit mix representations and resolution paths**

Add this variant and serialization branch in `ast.rs` while leaving `Named { soften: Option<u8> }` unchanged:

```rust
pub enum ColType {
    Player(usize),
    Named { color: NamedColor, soften: Option<u8> },
    Mix { source: NamedColor, target: NamedColor, pct: u8 },
}

// In Col::markup_col_type:
ColType::Mix { source, target, pct } => format!("mix({}, {}, {})", source, target, pct),
```

Add `parse_pct` in `parser.rs`, parsing only `0..=100` and returning a parser error otherwise. Use it for both `soften` and `mix`, so stored valid softens preserve their syntax but out-of-range inputs are rejected. Add `col_type_mix` before `col_type_soften` in `col_args`; it resolves both names through `resolve_named` and builds `ColType::Mix`.

```rust
fn parse_pct<Input>() -> impl Parser<Input, Output = u8>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(digit()).and_then(|s: String| {
        s.parse::<u8>()
            .ok()
            .filter(|pct| *pct <= 100)
            .ok_or_else(|| {
                <StreamErrorFor<Input>>::message_static_message("percentage must be 0 through 100")
            })
    })
}
```

```rust
fn col_type_mix<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("mix(")),
        many1::<String, _, _>(letter()),
        string(","),
        combine::optional(string(" ")),
        many1::<String, _, _>(letter()),
        string(","),
        combine::optional(string(" ")),
        parse_pct(),
        string(")"),
    )
        .and_then(|(_, source, _, _, target, _, _, pct, _)| {
            match (resolve_named(&source), resolve_named(&target)) {
                (Some(source), Some(target)) => Ok(ColType::Mix { source, target, pct }),
                _ => Err(<StreamErrorFor<Input>>::message_static_message("unknown named colour")),
            }
        })
}
```

In `transform.rs`, resolve explicit mixes after palette lookup:

```rust
ColType::Mix { source, target, pct } => brdgme_color::mix(
    palette.color(source),
    palette.color(target),
    pct,
),
```

Mirror the variant in `SemanticColType` and `Col::to_semantic`. In `html_class.rs`, format explicit mixes as `mix-{source}-{target}-{pct}`, and extend `markup_class_css` to emit structural rules for every `IN_USE_MIXES` entry using the matching `--mk-mix-*` variable and contrast variable.

- [ ] **Step 4: Run the markup test suite and formatting**

Run: `cargo test -p brdgme_markup`

Expected: all markup tests pass, including parsing, serializing, concrete resolution, semantic conversion, and CSS-token tests for mix.

Run: `cargo fmt --check -p brdgme_markup`

Expected: exits 0.

### Task 3: Wire Theme CSS, Pin Chrome Behaviour, And Update The Contract

**Files:**
- Modify: `rust/web/src/theme.rs:8-18,124-142,339-366`
- Modify: `rust/web/style/main.scss`
- Modify: `docs/authoring/THEMING.md:40-77,212-218,280-295`

**Interfaces:**
- Consumes: three-argument `palette_css_vars`, `IN_USE_MIXES`, and `mix` from Tasks 1-2.
- Produces: every static web theme block defines registered `mix-*` variables and background-tinted existing `soften-*` variables.
- Produces: authoring documentation for `mix(source, target, pct)` and the `soften` convenience wrapper.

- [ ] **Step 1: Write the failing web theme CSS regression test**

Add this unit test in `rust/web/src/theme.rs`:

```rust
#[test]
fn dracula_shared_chrome_surface_is_background_tinted() {
    let css = &*THEME_STYLE_CSS;
    let (_, dracula) = css
        .split_once("[data-theme=\"dracula\"]{")
        .expect("dracula theme block");
    let (dracula, _) = dracula.split_once("}\n").expect("theme block end");
    assert!(dracula.contains("--mk-soften-foreground-90: #3d3f49;"));
}
```

- [ ] **Step 2: Run the regression test to verify it fails**

Run: `SQLX_OFFLINE=true cargo test -p web --lib --features ssr dracula_shared_chrome_surface_is_background_tinted`

Expected: assertion failure because the current Dracula variable is `#4b4b4b`.

- [ ] **Step 3: Pass registered mix expressions into all theme blocks**

Import `IN_USE_MIXES` in `theme.rs`. Keep `CHROME_SOFTENS` and the local `softens` vector unchanged. Pass `IN_USE_MIXES` as the third argument to all three `palette_css_vars` calls:

```rust
palette_css_vars(light, &softens, IN_USE_MIXES)
palette_css_vars(dark, &softens, IN_USE_MIXES)
palette_css_vars(palette, &softens, IN_USE_MIXES)
```

Update `main.scss`'s shared `--mk-soften-foreground-86` references to
`--mk-soften-foreground-90`; these cover `.layout-header`, `.menu`,
`.game-meta`, shared borders, recent logs, suggestions, disabled inputs, and
current-turn chrome. Do not alter the orange/red state-cue variables at 86 or
the foreground hover variable at 96.

Rewrite the `THEMING.md` transform section to define `mix(source, target, pct)` as sRGB channel interpolation, including `0`/`100` endpoints. Define `soften(color, pct)` as `mix(color, BACKGROUND, pct)`. Replace HSL/lightness-only language, update Dracula's checkerboard explanation to describe background tinting and the 90/80 migration, and revise the Acquire pink migration from the historical HSL result `#f7bed4` to the sRGB-mixed `#f3d1de` at `soften(pink, 80)` on the light palette (moved from 75 because sRGB `soften(_, 75)` no longer clears the contrast floor on every registered theme). Preserve the historic plan unchanged; the approved `2026-07-14-mix-transform-design.md` remains its superseding record.

- [ ] **Step 4: Run verification and perform the bounded visual check**

Run: `SQLX_OFFLINE=true cargo test -p web --lib --features ssr theme::tests`

Expected: all theme unit tests pass, including the Dracula chrome surface assertion.

Run: `cargo test -p brdgme_color`

Run: `cargo test -p brdgme_markup`

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr`

Run: `cargo fmt --check`

Expected: every command exits 0.

With the running development site, select Dracula and verify the sidebar menu, game metadata panel, and narrow-screen header are blue-grey and visually close to `#282a36`; select brdgme light and verify Acquire's checkerboard is `#e6e6e6`/`#cccccc`. Switch themes without reloading and verify those chrome surfaces update immediately.

## Plan Review

- Spec coverage: Task 1 implements the sRGB resolver, inclusive endpoints, derived-surface CSS, and contrast coverage. Task 2 adds the reusable explicit transform across markup, semantic HTML, email/ANSI/plain resolution, and validation. Task 3 proves generated sidebar/header chrome values and updates the authoritative documentation without touching historical plans or CVD theme definitions.
- Placeholder scan: no TBD/TODO language or implicit test steps remain.
- Type consistency: `MixExpression`, `mix`, `ColType::Mix`, and `SemanticColType::Mix` are introduced before their downstream consumers and use the same `source`, `target`, and `pct` field names throughout.
