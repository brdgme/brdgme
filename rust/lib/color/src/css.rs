use crate::palette::{NamedColor, Palette};
use crate::{contrast, mix, soften};

/// Soften expressions actually used across game crates (audited from
/// acquire-1 and lords-of-vegas-1 `render.rs`). Kept as a single source of
/// truth so the web layer's generated CSS only carries the variables/classes
/// that are ever referenced.
///
/// Consolidated onto two stops, 80 and 90 (former 75/78/86 callers folded
/// in) - under the sRGB `mix`-based `soften`, 75 no longer clears the
/// contrast gate's 3:1 text floor for every registered theme (Solarized
/// Dark's grey against `soften(foreground, 75)` measured 2.86:1); 80 does.
pub const IN_USE_SOFTENS: &[(NamedColor, u8)] = &[
    (NamedColor::Foreground, 90),
    (NamedColor::Foreground, 80),
    (NamedColor::Pink, 80),
];

/// A `mix(source, target, pct)` expression, explicitly mixing one palette
/// slot toward another (as opposed to `soften`, which always mixes toward
/// the palette background).
pub type MixExpression = (NamedColor, NamedColor, u8);

/// Explicit mix expressions actually used across game crates. Empty for now
/// - no game currently emits an explicit `mix` expression. Kept as a single
///   source of truth so the web layer's generated CSS only carries the
///   variables/classes that are ever referenced.
pub const IN_USE_MIXES: &[MixExpression] = &[];

/// Generates `:root`-scope-free CSS custom property declarations (just the
/// `--mk-*: value;` lines, callers wrap them in a selector) for every named
/// palette slot plus its contrast counterpart, for each `soften_exprs` entry
/// plus its contrast counterpart, and for each `mix_exprs` entry plus its
/// contrast counterpart.
///
/// No player variables are emitted here - player-to-slot mapping is a
/// per-game concern; the web layer emits `--mk-player-{n}: var(--mk-{slot})`
/// itself, using the same `--mk-{name}` tokens this function defines.
pub fn palette_css_vars(
    palette: &Palette,
    soften_exprs: &[(NamedColor, u8)],
    mix_exprs: &[MixExpression],
) -> String {
    let mut buf = String::new();
    for named in NamedColor::ALL {
        let color = palette.color(named);
        buf.push_str(&format!("--mk-{}: {};\n", named, color));
        buf.push_str(&format!(
            "--mk-{}-contrast: {};\n",
            named,
            contrast(color, palette)
        ));
    }
    for &(named, pct) in soften_exprs {
        let base = palette.color(named);
        let softened = soften(base, pct, palette.background);
        buf.push_str(&format!("--mk-soften-{}-{}: {};\n", named, pct, softened));
        buf.push_str(&format!(
            "--mk-soften-{}-{}-contrast: {};\n",
            named,
            pct,
            contrast(softened, palette)
        ));
    }
    for &(source, target, pct) in mix_exprs {
        let mixed = mix(palette.color(source), palette.color(target), pct);
        let token = format!("mix-{source}-{target}-{pct}");
        buf.push_str(&format!("--mk-{}: {};\n", token, mixed));
        buf.push_str(&format!(
            "--mk-{}-contrast: {};\n",
            token,
            contrast(mixed, palette)
        ));
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::LIGHT;

    #[test]
    fn palette_css_vars_contains_named_and_contrast() {
        let css = palette_css_vars(&LIGHT, &[], &[]);
        assert!(css.contains(&format!("--mk-red: {};\n", LIGHT.red)));
        assert!(css.contains(&format!(
            "--mk-red-contrast: {};\n",
            contrast(LIGHT.red, &LIGHT)
        )));
    }

    #[test]
    fn palette_css_vars_contains_soften_and_contrast() {
        let css = palette_css_vars(&LIGHT, &[(NamedColor::Foreground, 86)], &[]);
        let softened = soften(LIGHT.foreground, 86, LIGHT.background);
        assert!(css.contains(&format!("--mk-soften-foreground-86: {};\n", softened)));
        assert!(css.contains(&format!(
            "--mk-soften-foreground-86-contrast: {};\n",
            contrast(softened, &LIGHT)
        )));
    }

    #[test]
    fn in_use_softens_matches_palette_css_vars() {
        let css = palette_css_vars(&LIGHT, IN_USE_SOFTENS, IN_USE_MIXES);
        for &(named, pct) in IN_USE_SOFTENS {
            assert!(css.contains(&format!("--mk-soften-{}-{}:", named, pct)));
        }
    }

    #[test]
    fn palette_css_vars_contains_mix_and_contrast() {
        let css = palette_css_vars(&LIGHT, &[], &[(NamedColor::Red, NamedColor::Blue, 50)]);
        assert!(css.contains("--mk-mix-red-blue-50: #765381;"));
        assert!(css.contains("--mk-mix-red-blue-50-contrast:"));
    }
}
