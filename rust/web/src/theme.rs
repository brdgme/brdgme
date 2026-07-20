//! Web-layer theming: static per-build theme CSS (SSR head), player colour
//! slot mapping (DB colour name -> `--mk-*` var), and the theme picker's
//! preview sample markup. See docs/authoring/THEMING.md for the underlying
//! palette/markup design; this module only wires that into the web crate.

use std::sync::LazyLock;

use brdgme_color::{
    IN_USE_MIXES, IN_USE_SOFTENS, NamedColor, ThemeCategory, palette_css_vars, themes,
};

/// Chrome-only soften expressions (main.scss surfaces: my-turn/finished/hover
/// tints) - kept separate from `IN_USE_SOFTENS` so the game-text contrast
/// gate in `brdgme_color` stays scoped to games; see THEMING.md.
const CHROME_SOFTENS: &[(NamedColor, u8)] = &[
    (NamedColor::Orange, 86),
    (NamedColor::Red, 86),
    (NamedColor::Foreground, 96),
];

/// Known theme slugs, paired with the display name from `brdgme_color::themes()`.
/// Hardcoded (rather than derived) so the cookie/`data-theme` value is a
/// stable, explicit contract; `theme_style_css_contains_all_themes` pins that
/// this stays in sync with `themes()`.
pub const THEME_SLUGS: &[(&str, &str)] = &[
    ("brdgme-light", "brdgme light"),
    ("brdgme-dark", "brdgme dark"),
    ("dracula", "dracula"),
    ("alucard", "alucard"),
    ("solarized-dark", "solarized dark"),
    ("solarized-light", "solarized light"),
    ("nord-dark", "nord dark"),
    ("nord-light", "nord light"),
    ("one-dark", "one dark"),
    ("one-light", "one light"),
    ("gruvbox-dark", "gruvbox dark"),
    ("gruvbox-light", "gruvbox light"),
    ("catppuccin-mocha", "catppuccin mocha"),
    ("catppuccin-latte", "catppuccin latte"),
    ("tokyo-night", "tokyo night"),
    ("tokyo-night-storm", "tokyo night storm"),
    ("tokyo-night-light", "tokyo night light"),
    ("night-owl", "night owl"),
    ("light-owl", "light owl"),
    ("synthwave-84", "synthwave 84"),
    ("papercolor-light", "papercolor light"),
    ("papercolor-dark", "papercolor dark"),
    ("monokai", "monokai"),
    ("darcula", "darcula"),
    ("vs-code-dark-plus", "vs code dark plus"),
    ("vs-code-dark-modern", "vs code dark modern"),
    ("brdgme-light-deuteranopia", "brdgme light deuteranopia"),
    ("brdgme-light-protanopia", "brdgme light protanopia"),
    ("brdgme-light-tritanopia", "brdgme light tritanopia"),
    ("brdgme-dark-deuteranopia", "brdgme dark deuteranopia"),
    ("brdgme-dark-protanopia", "brdgme dark protanopia"),
    ("brdgme-dark-tritanopia", "brdgme dark tritanopia"),
    ("modus-operandi-tritanopia", "modus operandi tritanopia"),
    ("modus-vivendi-tritanopia", "modus vivendi tritanopia"),
];

/// Canonical palette colour names, in palette order - the values stored in
/// `users.pref_colors`/`game_players.color`. Matches
/// `brdgme_color::Palette::player_colors()` slot order.
pub const PLAYER_COLOR_NAMES: [&str; 8] = [
    "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
];

/// Three distinct preferred colours for a brand-new account: a Fisher-Yates
/// shuffle of the full palette, truncated to 3 (so the trio is always valid
/// and duplicate-free). Randomised so new users don't all share one default.
pub fn random_pref_colors() -> Vec<String> {
    let mut colors: Vec<String> = PLAYER_COLOR_NAMES.iter().map(|s| s.to_string()).collect();
    for i in (1..colors.len()).rev() {
        let j = rand::random::<u32>() as usize % (i + 1);
        colors.swap(i, j);
    }
    colors.truncate(3);
    colors
}

/// Groups `THEME_SLUGS` by `brdgme_color::themes()`'s per-theme category,
/// sorted alphabetically by display name within each category, in category
/// order Default, Light, Dark, Deutan, Protan, Tritan (empty categories
/// omitted). Pure sort/group layer over the registry order that `themes()`/
/// `THEME_SLUGS` otherwise preserve.
pub fn grouped_themes() -> Vec<(ThemeCategory, Vec<(&'static str, &'static str)>)> {
    let categories = [
        ThemeCategory::Default,
        ThemeCategory::Light,
        ThemeCategory::Dark,
        ThemeCategory::Deutan,
        ThemeCategory::Protan,
        ThemeCategory::Tritan,
    ];
    categories
        .into_iter()
        .filter_map(|category| {
            let mut group: Vec<(&'static str, &'static str)> = themes()
                .iter()
                .filter(|(_, c, _)| *c == category)
                .filter_map(|(name, _, _)| THEME_SLUGS.iter().find(|(_, n)| n == name).copied())
                .collect();
            if group.is_empty() {
                return None;
            }
            group.sort_by_key(|(_, name)| *name);
            Some((category, group))
        })
        .collect()
}

/// Slugifies a theme's display name: lowercase, spaces -> hyphens.
pub fn slugify(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Whether `slug` is a valid theme selection (not "system", which is
/// represented by cookie absence rather than a slug).
pub fn is_known_slug(slug: &str) -> bool {
    THEME_SLUGS.iter().any(|(s, _)| *s == slug)
}

/// Builds the static, per-build theme stylesheet: markup colour classes, the
/// system-default (`:root` + `prefers-color-scheme`) vars, and one
/// `[data-theme="..."]` block per theme so every theme's vars are always
/// available for scoping to any element (not just `:root`) - this is what
/// lets the preview grid show every theme simultaneously.
fn build_theme_style_css() -> String {
    let mut css = String::new();

    css.push_str(&brdgme_markup::markup_class_css());

    // Base body colours; main.scss agrees (its chrome colours are all
    // var(--mk-*) / color-mix over the palette vars emitted below).
    css.push_str("body{background-color:var(--mk-background);color:var(--mk-foreground);}\n");

    let light = themes()
        .iter()
        .find(|(n, _, _)| *n == "brdgme light")
        .map(|(_, _, p)| *p)
        .expect("brdgme light theme must exist");
    let dark = themes()
        .iter()
        .find(|(n, _, _)| *n == "brdgme dark")
        .map(|(_, _, p)| *p)
        .expect("brdgme dark theme must exist");

    let softens: Vec<(NamedColor, u8)> = IN_USE_SOFTENS
        .iter()
        .chain(CHROME_SOFTENS)
        .copied()
        .collect();

    css.push_str(&format!(
        ":root{{{}}}\n",
        palette_css_vars(light, &softens, IN_USE_MIXES)
    ));
    css.push_str(&format!(
        "@media (prefers-color-scheme: dark){{:root:not([data-theme]){{{}}}}}\n",
        palette_css_vars(dark, &softens, IN_USE_MIXES)
    ));

    for (name, _, palette) in themes() {
        let slug = slugify(name);
        css.push_str(&format!(
            "[data-theme=\"{}\"]{{{}}}\n",
            slug,
            palette_css_vars(palette, &softens, IN_USE_MIXES)
        ));
    }

    css
}

/// Built once per process; the theme set is fixed at compile time.
pub static THEME_STYLE_CSS: LazyLock<String> = LazyLock::new(build_theme_style_css);

/// Maps a `game_players.color`/`users.pref_colors` slot name to the lowercase
/// token used in `--mk-{slot}` vars. Legacy rows may still say "Amber" or
/// "BlueGrey" (pre-2026-07 palette); those map onto the slots that inherited
/// their position ("Amber" -> orange, "BlueGrey" -> cyan). Anything else
/// (defensive, should not happen) falls back to "grey".
pub fn slot_from_color_name(name: &str) -> &'static str {
    match name {
        "Green" => "green",
        "Red" => "red",
        "Blue" => "blue",
        "Orange" | "Amber" => "orange",
        "Purple" => "purple",
        "Brown" => "brown",
        "Cyan" | "BlueGrey" => "cyan",
        "Pink" => "pink",
        _ => "grey",
    }
}

/// Builds the `--mk-player-{n}`/`--mk-player-{n}-contrast` var declarations
/// for a game's player slots, in position order - the inline `style` a board/
/// log container needs so markup's `mk-fg-player-{n}`/`mk-bg-player-{n}`
/// classes resolve to that game's actual player colours.
pub fn player_style_vars(slots: &[&str]) -> String {
    slots
        .iter()
        .enumerate()
        .map(|(n, slot)| {
            format!(
                "--mk-player-{n}: var(--mk-{slot}); --mk-player-{n}-contrast: var(--mk-{slot}-contrast);"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Two rows of five 5-space background swatches - the 10 accent colours in
/// `NamedColor::ALL` order (Foreground/Background excluded; the tile itself
/// already shows them). No text: the swatch blocks read purely as colour.
const SAMPLE_MARKUP: &str = "{{bg red}}     {{/bg}}{{bg green}}     {{/bg}}\
{{bg blue}}     {{/bg}}{{bg yellow}}     {{/bg}}{{bg purple}}     {{/bg}}\n\
{{bg cyan}}     {{/bg}}{{bg pink}}     {{/bg}}{{bg orange}}     {{/bg}}\
{{bg brown}}     {{/bg}}{{bg grey}}     {{/bg}}";

fn build_sample_html() -> String {
    let (nodes, _) = brdgme_markup::from_string(SAMPLE_MARKUP).unwrap_or_default();
    let tnodes = brdgme_markup::transform_semantic(&nodes, &[]);
    brdgme_markup::html_class(&tnodes)
}

/// Two rows of five solid colour swatches (see `SAMPLE_MARKUP`), rendered
/// once via `html_class`/`transform_semantic`; shown on every theme preview
/// tile.
pub static SAMPLE_HTML: LazyLock<String> = LazyLock::new(build_sample_html);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_slugs_match_brdgme_color_themes() {
        let names: Vec<String> = themes().iter().map(|(n, _, _)| slugify(n)).collect();
        let ours: Vec<&str> = THEME_SLUGS.iter().map(|(s, _)| *s).collect();
        assert_eq!(names, ours);
    }

    #[test]
    fn grouped_themes_category_order_and_sorting() {
        let groups = grouped_themes();
        let cats: Vec<ThemeCategory> = groups.iter().map(|(c, _)| *c).collect();
        // Only categories present in the registry appear, in this order.
        let mut expected_order = vec![
            ThemeCategory::Default,
            ThemeCategory::Light,
            ThemeCategory::Dark,
            ThemeCategory::Deutan,
            ThemeCategory::Protan,
            ThemeCategory::Tritan,
        ];
        expected_order.retain(|c| cats.contains(c));
        assert_eq!(cats, expected_order);

        for (_, themes_in_group) in &groups {
            let mut sorted = themes_in_group.clone();
            sorted.sort_by_key(|(_, name)| *name);
            assert_eq!(themes_in_group, &sorted, "group not alphabetically sorted");
        }

        let total: usize = groups.iter().map(|(_, g)| g.len()).sum();
        assert_eq!(total, THEME_SLUGS.len());

        let default_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Default)
            .expect("Default category must be present")
            .1
            .clone();
        assert!(
            default_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-light")
        );
        assert!(default_group.iter().any(|(slug, _)| *slug == "brdgme-dark"));

        let light_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Light)
            .expect("Light category must be present")
            .1
            .clone();
        assert!(light_group.iter().any(|(slug, _)| *slug == "alucard"));
        assert!(light_group.iter().any(|(slug, _)| *slug == "gruvbox-light"));
        assert!(!light_group.iter().any(|(slug, _)| *slug == "gruvbox-dark"));

        let dark_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Dark)
            .expect("Dark category must be present")
            .1
            .clone();
        assert!(dark_group.iter().any(|(slug, _)| *slug == "dracula"));
        assert!(dark_group.iter().any(|(slug, _)| *slug == "gruvbox-dark"));
        assert!(!dark_group.iter().any(|(slug, _)| *slug == "gruvbox-light"));

        let deutan_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Deutan)
            .expect("Deutan category must be present")
            .1
            .clone();
        assert!(
            deutan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-light-deuteranopia")
        );
        assert!(
            deutan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-dark-deuteranopia")
        );
        assert!(
            deutan_group
                .iter()
                .all(|(slug, _)| slug.contains("deuteranopia"))
        );

        let protan_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Protan)
            .expect("Protan category must be present")
            .1
            .clone();
        assert!(
            protan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-light-protanopia")
        );
        assert!(
            protan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-dark-protanopia")
        );
        assert!(
            protan_group
                .iter()
                .all(|(slug, _)| slug.contains("protanopia"))
        );

        let tritan_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Tritan)
            .expect("Tritan category must be present")
            .1
            .clone();
        assert!(
            tritan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-light-tritanopia")
        );
        assert!(
            tritan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-dark-tritanopia")
        );
        assert!(
            !tritan_group
                .iter()
                .any(|(slug, _)| slug.contains("deuteranopia") || slug.contains("protanopia"))
        );
    }

    #[test]
    fn theme_style_css_contains_expected_rules() {
        let css = &*THEME_STYLE_CSS;
        assert!(css.contains("--mk-red"));
        assert!(css.contains("[data-theme=\"dracula\"]"));
        assert!(css.contains("[data-theme=\"brdgme-dark\"]"));
        assert!(css.contains("prefers-color-scheme: dark"));
        assert!(css.contains(".mk-fg-player-0{color:var(--mk-player-0)}"));
        assert!(css.contains("--mk-soften-orange-86"));
    }

    #[test]
    fn chrome_softens_meet_contrast_floor() {
        use brdgme_color::{contrast_ratio, soften};
        for (theme_name, _, palette) in themes() {
            for &(named, pct) in CHROME_SOFTENS {
                let surface = soften(palette.color(named), pct, palette.background);
                let ratio = contrast_ratio(palette.foreground, surface);
                assert!(
                    ratio >= 3.0,
                    "[{}] foreground vs soften({}, {}): {:.2} < 3.0",
                    theme_name,
                    named,
                    pct,
                    ratio
                );
            }
        }
    }

    #[test]
    fn dracula_shared_chrome_surface_is_background_tinted() {
        let css = &*THEME_STYLE_CSS;
        let (_, dracula) = css
            .split_once("[data-theme=\"dracula\"]{")
            .expect("dracula theme block");
        let (dracula, _) = dracula.split_once("}\n").expect("theme block end");
        assert!(dracula.contains("--mk-soften-foreground-90: #3d3f49;"));
    }

    #[test]
    fn slot_from_color_name_maps_legacy_names() {
        assert_eq!(slot_from_color_name("Amber"), "orange");
        assert_eq!(slot_from_color_name("BlueGrey"), "cyan");
        assert_eq!(slot_from_color_name("Green"), "green");
        assert_eq!(slot_from_color_name("Unknown"), "grey");
    }

    #[test]
    fn player_style_vars_formats_correctly() {
        let css = player_style_vars(&["green", "red"]);
        assert!(css.contains("--mk-player-0: var(--mk-green);"));
        assert!(css.contains("--mk-player-0-contrast: var(--mk-green-contrast);"));
        assert!(css.contains("--mk-player-1: var(--mk-red);"));
    }

    #[test]
    fn sample_html_renders_expected_pieces() {
        let html = &*SAMPLE_HTML;
        // The 10 accent colours from NamedColor::ALL, in order; Foreground
        // and Background are excluded (the tile itself shows them).
        for slot in [
            "red", "green", "blue", "yellow", "purple", "cyan", "pink", "orange", "brown", "grey",
        ] {
            assert!(html.contains(&format!("mk-bg-{slot}")), "missing bg {slot}");
        }
        assert!(
            !html.contains("mk-bg-foreground") && !html.contains("mk-bg-background"),
            "foreground/background must be excluded"
        );
        assert!(html.contains("     "), "swatches are 5-space blocks");
        assert!(!html.contains("mk-fg-"), "no text/fg in the sample");
        assert!(!html.contains("Green"), "no colour names in the sample");
        assert_eq!(
            html.matches('\n').count(),
            1,
            "exactly two rows separated by one newline"
        );
    }

    #[test]
    fn random_pref_colors_three_distinct_valid() {
        for _ in 0..50 {
            let colors = random_pref_colors();
            assert_eq!(colors.len(), 3);
            assert!(
                colors
                    .iter()
                    .all(|c| PLAYER_COLOR_NAMES.contains(&c.as_str()))
            );
            assert_ne!(colors[0], colors[1]);
            assert_ne!(colors[0], colors[2]);
            assert_ne!(colors[1], colors[2]);
        }
    }
}
