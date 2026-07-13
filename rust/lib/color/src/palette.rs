use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Color, ColorError};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum NamedColor {
    Red,
    Green,
    Blue,
    Yellow,
    Purple,
    Cyan,
    Pink,
    Orange,
    Brown,
    Grey,
    Foreground,
    Background,
}

impl NamedColor {
    pub const ALL: [NamedColor; 12] = [
        NamedColor::Red,
        NamedColor::Green,
        NamedColor::Blue,
        NamedColor::Yellow,
        NamedColor::Purple,
        NamedColor::Cyan,
        NamedColor::Pink,
        NamedColor::Orange,
        NamedColor::Brown,
        NamedColor::Grey,
        NamedColor::Foreground,
        NamedColor::Background,
    ];
}

impl fmt::Display for NamedColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            NamedColor::Red => "red",
            NamedColor::Green => "green",
            NamedColor::Blue => "blue",
            NamedColor::Yellow => "yellow",
            NamedColor::Purple => "purple",
            NamedColor::Cyan => "cyan",
            NamedColor::Pink => "pink",
            NamedColor::Orange => "orange",
            NamedColor::Brown => "brown",
            NamedColor::Grey => "grey",
            NamedColor::Foreground => "foreground",
            NamedColor::Background => "background",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for NamedColor {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<Self, ColorError> {
        match s.to_lowercase().as_str() {
            "red" => Ok(NamedColor::Red),
            "green" => Ok(NamedColor::Green),
            "blue" => Ok(NamedColor::Blue),
            "yellow" => Ok(NamedColor::Yellow),
            "purple" => Ok(NamedColor::Purple),
            "cyan" => Ok(NamedColor::Cyan),
            "pink" => Ok(NamedColor::Pink),
            "orange" => Ok(NamedColor::Orange),
            "brown" => Ok(NamedColor::Brown),
            "grey" => Ok(NamedColor::Grey),
            "foreground" => Ok(NamedColor::Foreground),
            "background" => Ok(NamedColor::Background),
            _ => Err(ColorError::Parse {
                message: format!(r##"could not find named color "{}""##, s),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub red: Color,
    pub green: Color,
    pub blue: Color,
    pub yellow: Color,
    pub purple: Color,
    pub cyan: Color,
    pub pink: Color,
    pub orange: Color,
    pub brown: Color,
    pub grey: Color,
    pub foreground: Color,
    pub background: Color,
}

impl Palette {
    pub fn color(&self, named: NamedColor) -> Color {
        match named {
            NamedColor::Red => self.red,
            NamedColor::Green => self.green,
            NamedColor::Blue => self.blue,
            NamedColor::Yellow => self.yellow,
            NamedColor::Purple => self.purple,
            NamedColor::Cyan => self.cyan,
            NamedColor::Pink => self.pink,
            NamedColor::Orange => self.orange,
            NamedColor::Brown => self.brown,
            NamedColor::Grey => self.grey,
            NamedColor::Foreground => self.foreground,
            NamedColor::Background => self.background,
        }
    }

    pub fn player_colors(&self) -> [Color; 8] {
        [
            self.green,
            self.red,
            self.blue,
            self.orange,
            self.purple,
            self.brown,
            self.cyan,
            self.pink,
        ]
    }

    pub fn player_color(&self, player: usize) -> Color {
        let colors = self.player_colors();
        colors[player % colors.len()]
    }
}

pub static LIGHT: Palette = Palette {
    red: Color {
        r: 211,
        g: 47,
        b: 47,
    },
    green: Color {
        r: 56,
        g: 142,
        b: 60,
    },
    blue: Color {
        r: 25,
        g: 118,
        b: 210,
    },
    yellow: Color {
        r: 251,
        g: 192,
        b: 45,
    },
    purple: Color {
        r: 123,
        g: 31,
        b: 162,
    },
    cyan: Color {
        r: 0,
        g: 151,
        b: 167,
    },
    pink: Color {
        r: 194,
        g: 24,
        b: 91,
    },
    orange: Color {
        r: 245,
        g: 124,
        b: 0,
    },
    brown: Color {
        r: 93,
        g: 64,
        b: 55,
    },
    grey: Color {
        r: 97,
        g: 97,
        b: 97,
    },
    foreground: Color { r: 0, g: 0, b: 0 },
    background: Color {
        r: 255,
        g: 255,
        b: 255,
    },
};

pub static DARK: Palette = Palette {
    red: Color {
        r: 239,
        g: 83,
        b: 80,
    },
    green: Color {
        r: 102,
        g: 187,
        b: 106,
    },
    blue: Color {
        r: 66,
        g: 165,
        b: 245,
    },
    yellow: Color {
        r: 255,
        g: 238,
        b: 88,
    },
    purple: Color {
        r: 171,
        g: 71,
        b: 188,
    },
    cyan: Color {
        r: 38,
        g: 198,
        b: 218,
    },
    pink: Color {
        r: 236,
        g: 64,
        b: 122,
    },
    orange: Color {
        r: 255,
        g: 167,
        b: 38,
    },
    brown: Color {
        r: 143,
        g: 83,
        b: 61,
    },
    grey: Color {
        r: 158,
        g: 158,
        b: 158,
    },
    foreground: Color {
        r: 255,
        g: 255,
        b: 255,
    },
    background: Color {
        r: 18,
        g: 18,
        b: 18,
    },
};

/// Dracula theme. RED/GREEN/YELLOW/PURPLE/CYAN/PINK/ORANGE are the
/// official Dracula palette values. BLUE and BROWN are derived tones (see
/// docs/authoring/THEMING.md); BLUE/BROWN/GREY/FOREGROUND were nudged from
/// their initial derived/official values to pass the contrast gate (see
/// `tests::gate` and the phase-27 report for the before/after values and
/// which check forced each change).
pub static DRACULA: Palette = Palette {
    red: Color {
        r: 255,
        g: 85,
        b: 85,
    },
    green: Color {
        r: 80,
        g: 250,
        b: 123,
    },
    blue: Color {
        r: 112,
        g: 140,
        b: 245,
    },
    yellow: Color {
        r: 241,
        g: 250,
        b: 140,
    },
    purple: Color {
        r: 189,
        g: 147,
        b: 249,
    },
    cyan: Color {
        r: 139,
        g: 233,
        b: 253,
    },
    pink: Color {
        r: 255,
        g: 121,
        b: 198,
    },
    orange: Color {
        r: 255,
        g: 184,
        b: 108,
    },
    brown: Color {
        r: 188,
        g: 137,
        b: 103,
    },
    grey: Color {
        r: 184,
        g: 191,
        b: 214,
    },
    foreground: Color {
        r: 248,
        g: 248,
        b: 248,
    },
    background: Color {
        r: 40,
        g: 42,
        b: 54,
    },
};

/// The set of registered themes, in display order.
pub fn themes() -> &'static [(&'static str, &'static Palette)] {
    static THEMES: [(&str, &Palette); 3] = [
        ("brdgme light", &LIGHT),
        ("brdgme dark", &DARK),
        ("dracula", &DRACULA),
    ];
    &THEMES
}

/// Converts RGB (0..=255 each) to HSL, with h in 0..360, s and l in 0..=1.
fn rgb_to_hsl(c: Color) -> (f64, f64, f64) {
    let r = c.r as f64 / 255.0;
    let g = c.g as f64 / 255.0;
    let b = c.b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f64::EPSILON {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, l)
}

fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    let mut t = t;
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Rounds a 0..=255 scale value half-up.
fn round_u8(v: f64) -> u8 {
    (v + 0.5).floor().clamp(0.0, 255.0) as u8
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    if s.abs() < f64::EPSILON {
        let v = round_u8(l * 255.0);
        return Color { r: v, g: v, b: v };
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;

    let r = hue_to_rgb(p, q, h_norm + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h_norm);
    let b = hue_to_rgb(p, q, h_norm - 1.0 / 3.0);

    Color {
        r: round_u8(r * 255.0),
        g: round_u8(g * 255.0),
        b: round_u8(b * 255.0),
    }
}

/// Derives a muted surface tint from `color`, moving its lightness toward
/// `background`'s lightness by `pct`% (1..=99), keeping hue and saturation.
pub fn soften(color: Color, pct: u8, background: Color) -> Color {
    let pct = pct.clamp(1, 99);
    let (h, s, l) = rgb_to_hsl(color);
    let (_, _, l_bg) = rgb_to_hsl(background);
    let l_new = l + (l_bg - l) * (pct as f64 / 100.0);
    hsl_to_rgb(h, s, l_new)
}

fn srgb_channel_to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(c: Color) -> f64 {
    0.2126 * srgb_channel_to_linear(c.r)
        + 0.7152 * srgb_channel_to_linear(c.g)
        + 0.0722 * srgb_channel_to_linear(c.b)
}

/// WCAG contrast ratio between two colours (1.0..=21.0).
pub fn contrast_ratio(a: Color, b: Color) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Returns whichever of the palette's Foreground or Background has the
/// higher WCAG contrast ratio against `color`.
pub fn contrast(color: Color, palette: &Palette) -> Color {
    let fg_ratio = contrast_ratio(color, palette.foreground);
    let bg_ratio = contrast_ratio(color, palette.background);
    if fg_ratio >= bg_ratio {
        palette.foreground
    } else {
        palette.background
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soften_exactness() {
        assert_eq!(
            soften(LIGHT.foreground, 86, LIGHT.background).hex(),
            "#dbdbdb"
        );
        assert_eq!(
            soften(LIGHT.foreground, 75, LIGHT.background).hex(),
            "#bfbfbf"
        );
        assert_eq!(soften(LIGHT.pink, 75, LIGHT.background).hex(), "#f7bed4");
    }

    #[test]
    fn contrast_picks_correct_side() {
        assert_eq!(contrast(LIGHT.yellow, &LIGHT), LIGHT.foreground);
        assert_eq!(contrast(LIGHT.brown, &LIGHT), LIGHT.background);
    }

    #[test]
    fn player_color_order() {
        assert_eq!(LIGHT.player_color(0), LIGHT.green);
        assert_eq!(LIGHT.player_color(1), LIGHT.red);
        assert_eq!(LIGHT.player_color(3), LIGHT.orange);
        assert_eq!(LIGHT.player_color(3), Color::from_hex("#f57c00").unwrap());
        assert_eq!(LIGHT.player_color(7), LIGHT.pink);
        assert_eq!(LIGHT.player_color(8), LIGHT.player_color(0));
    }

    #[test]
    fn named_color_round_trip() {
        for nc in NamedColor::ALL {
            let s = nc.to_string();
            let parsed: NamedColor = s.parse().unwrap();
            assert_eq!(parsed, nc);
            let upper: NamedColor = s.to_uppercase().parse().unwrap();
            assert_eq!(upper, nc);
        }
    }

    // --- Contrast gate (docs/authoring/THEMING.md "Contrast requirements") ---
    //
    // The 9 hues that are ever used as bare text against BACKGROUND. YELLOW
    // and ORANGE are excluded: the colour audit behind THEMING.md found no
    // game renders them as literal foreground text (they're always dice/tile
    // *backgrounds*, with `contrast` supplying the readable text on top - see
    // check `gate_contrast_transform`). Including them here would make LIGHT
    // itself fail (Material yellow/orange on white sit at ~1.7:1 and ~2.7:1),
    // which the task brief flags as the check being miscalibrated, not LIGHT
    // being wrong.
    const TEXT_HUES: [NamedColor; 7] = [
        NamedColor::Red,
        NamedColor::Green,
        NamedColor::Blue,
        NamedColor::Purple,
        NamedColor::Cyan,
        NamedColor::Pink,
        NamedColor::Brown,
    ];

    // All 9 hues, used for the `contrast` transform check and the specific
    // hue-pair distinctness checks.
    const ALL_HUES: [NamedColor; 9] = [
        NamedColor::Red,
        NamedColor::Green,
        NamedColor::Blue,
        NamedColor::Yellow,
        NamedColor::Purple,
        NamedColor::Cyan,
        NamedColor::Pink,
        NamedColor::Orange,
        NamedColor::Brown,
    ];

    const TEXT_FLOOR: f64 = 3.0;
    const TRANSFORM_FLOOR: f64 = 4.5;
    // LIGHT's minimum pairwise player deltaE (brown vs grey) measures ~19.02
    // (see below); LIGHT is definitionally valid, so the threshold is set at
    // a round number safely below that.
    const DISTINCT_DELTA_E: f64 = 15.0;

    fn in_use_surfaces(palette: &Palette) -> Vec<(String, Color)> {
        crate::css::IN_USE_SOFTENS
            .iter()
            .map(|&(named, pct)| {
                (
                    format!("soften({}, {})", named, pct),
                    soften(palette.color(named), pct, palette.background),
                )
            })
            .collect()
    }

    /// sRGB (D65) -> CIE Lab, for CIE76 deltaE.
    fn rgb_to_lab(c: Color) -> (f64, f64, f64) {
        fn lin(v: u8) -> f64 {
            let c = v as f64 / 255.0;
            if c > 0.04045 {
                ((c + 0.055) / 1.055).powf(2.4)
            } else {
                c / 12.92
            }
        }
        let r = lin(c.r);
        let g = lin(c.g);
        let b = lin(c.b);
        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
        let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

        fn f(t: f64) -> f64 {
            if t > 0.008856 {
                t.cbrt()
            } else {
                7.787 * t + 16.0 / 116.0
            }
        }
        let (xn, yn, zn) = (0.95047_f64, 1.0_f64, 1.08883_f64);
        let (fx, fy, fz) = (f(x / xn), f(y / yn), f(z / zn));
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);
        (l, a, b)
    }

    /// CIE76 deltaE (Euclidean distance in CIELAB).
    fn delta_e(a: Color, b: Color) -> f64 {
        let (l1, a1, b1) = rgb_to_lab(a);
        let (l2, a2, b2) = rgb_to_lab(b);
        ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
    }

    #[test]
    fn gate_light_minimum_player_delta_e() {
        // Documents the measurement DISTINCT_DELTA_E was calibrated from.
        let min = min_pairwise_player_delta_e(&LIGHT);
        assert!(
            min.0 >= 19.0,
            "LIGHT minimum player deltaE dropped below the recorded ~19.02 \
             (now {:.2} for {}/{}); recalibrate DISTINCT_DELTA_E if this is intentional",
            min.0,
            min.1,
            min.2
        );
    }

    fn min_pairwise_player_delta_e(palette: &Palette) -> (f64, String, String) {
        let players = palette.player_colors();
        const PLAYER_NAMES: [&str; 8] = [
            "green", "red", "blue", "orange", "purple", "brown", "cyan", "pink",
        ];
        let mut min: Option<(f64, String, String)> = None;
        for i in 0..players.len() {
            for j in (i + 1)..players.len() {
                let d = delta_e(players[i], players[j]);
                if min.as_ref().map(|m| d < m.0).unwrap_or(true) {
                    min = Some((d, PLAYER_NAMES[i].to_string(), PLAYER_NAMES[j].to_string()));
                }
            }
        }
        for (i, &p) in players.iter().enumerate() {
            for (extra, name) in [(palette.grey, "grey"), (palette.foreground, "foreground")] {
                let d = delta_e(p, extra);
                if min.as_ref().map(|m| d < m.0).unwrap_or(true) {
                    min = Some((d, PLAYER_NAMES[i].to_string(), name.to_string()));
                }
            }
        }
        min.expect("player_colors is non-empty")
    }

    #[test]
    fn gate_contrast_all_themes() {
        for (theme_name, palette) in crate::themes() {
            let surfaces = in_use_surfaces(palette);

            // 3a: FOREGROUND, GREY, and every hue used as text reach >= 3:1
            // against BACKGROUND and every in-use softened surface (hues
            // only against BACKGROUND - see TEXT_HUES doc comment).
            for who in [NamedColor::Foreground, NamedColor::Grey] {
                let c = palette.color(who);
                let r = contrast_ratio(c, palette.background);
                assert!(
                    r >= TEXT_FLOOR,
                    "[{}] {} vs background: {:.2} < {}",
                    theme_name,
                    who,
                    r,
                    TEXT_FLOOR
                );
                for (surface_name, surface) in &surfaces {
                    let r = contrast_ratio(c, *surface);
                    assert!(
                        r >= TEXT_FLOOR,
                        "[{}] {} vs {}: {:.2} < {}",
                        theme_name,
                        who,
                        surface_name,
                        r,
                        TEXT_FLOOR
                    );
                }
            }
            for hue in TEXT_HUES {
                let c = palette.color(hue);
                let r = contrast_ratio(c, palette.background);
                assert!(
                    r >= TEXT_FLOOR,
                    "[{}] {} (text) vs background: {:.2} < {}",
                    theme_name,
                    hue,
                    r,
                    TEXT_FLOOR
                );
            }

            // 3b: `contrast` output reaches >= 4.5:1 against every hue, GREY,
            // and every in-use softened surface.
            let mut transform_targets: Vec<(String, Color)> = ALL_HUES
                .iter()
                .map(|&h| (h.to_string(), palette.color(h)))
                .collect();
            transform_targets.push((NamedColor::Grey.to_string(), palette.grey));
            transform_targets.extend(surfaces.iter().cloned());
            for (name, c) in transform_targets {
                let cc = contrast(c, palette);
                let r = contrast_ratio(c, cc);
                assert!(
                    r >= TRANSFORM_FLOOR,
                    "[{}] contrast({}) = {:.2} < {}",
                    theme_name,
                    name,
                    r,
                    TRANSFORM_FLOOR
                );
            }

            // 3c: all 8 player colours pairwise distinguishable, and each
            // distinguishable from GREY and FOREGROUND.
            let (min_delta, a, b) = min_pairwise_player_delta_e(palette);
            assert!(
                min_delta >= DISTINCT_DELTA_E,
                "[{}] player colours {}/{} too close: deltaE {:.2} < {}",
                theme_name,
                a,
                b,
                min_delta,
                DISTINCT_DELTA_E
            );

            // 3d: specific hue pairs games rely on staying distinct.
            let pairs = [
                (NamedColor::Red, NamedColor::Orange),
                (NamedColor::Orange, NamedColor::Yellow),
                (NamedColor::Red, NamedColor::Yellow),
                (NamedColor::Blue, NamedColor::Cyan),
                (NamedColor::Grey, NamedColor::Foreground),
                (NamedColor::Grey, NamedColor::Brown),
            ];
            for (x, y) in pairs {
                let d = delta_e(palette.color(x), palette.color(y));
                assert!(
                    d >= DISTINCT_DELTA_E,
                    "[{}] {}/{} too close: deltaE {:.2} < {}",
                    theme_name,
                    x,
                    y,
                    d,
                    DISTINCT_DELTA_E
                );
            }
        }
    }
}
