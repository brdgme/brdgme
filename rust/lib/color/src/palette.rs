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

/// Alucard theme (Dracula's official light variant, "Alucard Classic").
/// BACKGROUND, FOREGROUND, RED, GREEN, YELLOW, PURPLE, CYAN, PINK, ORANGE are
/// the official Alucard Classic values from draculatheme.com/spec (verified
/// 2026-07-13). GREY takes the official Comment/Current Line colour
/// (`#6c664b`). BLUE and BROWN are derived, as Alucard has no native
/// blue/brown accent (same approach as DRACULA):
/// - BLUE sits at a hue between CYAN (~198°) and PURPLE (~252°), landing at
///   ~223°, with saturation/lightness picked to sit visually alongside the
///   other Alucard hues -> `#203f97` (32,68,151).
/// - BROWN is ORANGE (`#a34d14`, HSL ~24°/78%/36%) with saturation and
///   lightness reduced in the same proportion DRACULA's derived BROWN used
///   relative to DRACULA's ORANGE (~39% of saturation, ~77% of lightness),
///   hue nudged slightly warmer -> `#5d3e32` (93,62,50).
///
/// Unlike DRACULA, none of these values (official or derived) needed nudging
/// to pass the contrast gate (`tests::gate_contrast_all_themes`) - Alucard's
/// darker-on-light hues clear the text/transform/distinctness floors as
/// derived.
pub static ALUCARD: Palette = Palette {
    red: Color {
        r: 203,
        g: 58,
        b: 42,
    },
    green: Color {
        r: 20,
        g: 113,
        b: 10,
    },
    blue: Color {
        r: 32,
        g: 68,
        b: 151,
    },
    yellow: Color {
        r: 132,
        g: 110,
        b: 21,
    },
    purple: Color {
        r: 100,
        g: 74,
        b: 201,
    },
    cyan: Color {
        r: 3,
        g: 106,
        b: 150,
    },
    pink: Color {
        r: 163,
        g: 20,
        b: 77,
    },
    orange: Color {
        r: 163,
        g: 77,
        b: 20,
    },
    brown: Color {
        r: 93,
        g: 62,
        b: 50,
    },
    grey: Color {
        r: 108,
        g: 102,
        b: 75,
    },
    foreground: Color {
        r: 31,
        g: 31,
        b: 31,
    },
    background: Color {
        r: 255,
        g: 251,
        b: 235,
    },
};

/// Solarized Dark (Ethan Schoonover, ethanschoonover.com/solarized;
/// verified 2026-07-13 against the same source). RED (`base03`'s `red`),
/// GREEN, YELLOW, CYAN, ORANGE are the official Solarized accent values
/// unchanged. PURPLE and BLUE are Solarized's `violet`/`blue` accents,
/// nudged to pass the contrast gate (see below). PINK is Solarized's
/// `magenta`, unchanged. BACKGROUND is the official `base03`
/// (`#002b36`).
///
/// FOREGROUND and GREY are tuned, not official Solarized values: official
/// dark-mode body text (`base0`/`base1`) tops out around 5.6:1 against
/// `base03` and is far too dim to carry the `contrast` transform's 4.5:1
/// floor against several accents (red/orange/magenta/purple/blue all
/// landed under 4.5:1 in testing). FOREGROUND is tuned to pure white
/// (`#ffffff`, up from `base3`'s `#fdf6e3`, which still fell short) and
/// GREY is tuned to a Solarized-hued (`~196°`) light blue-grey
/// (`#9aabb1`) lighter than official `base00`/`base01`, both driven
/// purely by `tests::gate_contrast_all_themes`.
///
/// PURPLE (`violet` `#6c71c4`) is darkened slightly (`l -1.5%`) to
/// `#676cc2` so `contrast` (now landing on FOREGROUND) clears 4.5:1;
/// BLUE (`#268bd2`) is lightened slightly (`l +5%`) to `#3797db` so
/// `contrast` (landing on BACKGROUND) clears 4.5:1 - both moves keep the
/// hue and saturation, changing only lightness, and remain clearly the
/// same accent.
///
/// BROWN has no Solarized accent (the palette has no brown); it is
/// derived at hue 28° (between Solarized's `orange` 17.6° and `yellow`
/// 45.4°) with reduced saturation/lightness (`#ac6039`), chosen to clear
/// the contrast and distinctness gates against BACKGROUND, GREY, and
/// ORANGE.
pub static SOLARIZED_DARK: Palette = Palette {
    red: Color {
        r: 220,
        g: 50,
        b: 47,
    },
    green: Color {
        r: 133,
        g: 153,
        b: 0,
    },
    blue: Color {
        r: 55,
        g: 151,
        b: 219,
    },
    yellow: Color {
        r: 181,
        g: 137,
        b: 0,
    },
    purple: Color {
        r: 103,
        g: 108,
        b: 194,
    },
    cyan: Color {
        r: 42,
        g: 161,
        b: 152,
    },
    pink: Color {
        r: 211,
        g: 54,
        b: 130,
    },
    orange: Color {
        r: 203,
        g: 75,
        b: 22,
    },
    brown: Color {
        r: 172,
        g: 96,
        b: 57,
    },
    grey: Color {
        r: 154,
        g: 171,
        b: 177,
    },
    foreground: Color {
        r: 255,
        g: 255,
        b: 255,
    },
    background: Color { r: 0, g: 43, b: 54 },
};

/// Solarized Light (Ethan Schoonover, ethanschoonover.com/solarized;
/// verified 2026-07-13). Shares the same 8 accent hues as SOLARIZED_DARK
/// (RED/GREEN/YELLOW/PURPLE/CYAN/PINK/ORANGE/BLUE = Solarized's
/// red/green/yellow/violet/cyan/magenta/orange/blue), each an
/// independently-tuned palette (a hue passing on one background can fail
/// on the other). BACKGROUND is the official `base3` (`#fdf6e3`).
///
/// GREEN and CYAN are tuned darker than their official values: official
/// `green`/`cyan` sit just under the 3:1 text floor against `base3`
/// (2.97:1 / 2.93:1). Both are darkened slightly (`l -3%`, hue/saturation
/// unchanged) to `#788a00` / `#279589`, clearing the floor with margin.
///
/// FOREGROUND and GREY are tuned, not official Solarized values, for the
/// same reason as SOLARIZED_DARK: official light-mode text
/// (`base00`/`base01`) doesn't give `contrast` enough range against the
/// accents. FOREGROUND is tuned to pure black (`#000000`, down from
/// `base00`'s `#657b83`) and GREY to a Solarized-hued (`196°`) darker
/// blue-grey (`#4c5f67`), both driven by
/// `tests::gate_contrast_all_themes`.
///
/// BROWN is derived the same way as SOLARIZED_DARK (hue 28°, between
/// `orange` and `yellow`) but darker/less saturated to read as text on a
/// light background (`#734623`), chosen to clear the contrast and
/// distinctness gates.
pub static SOLARIZED_LIGHT: Palette = Palette {
    red: Color {
        r: 220,
        g: 50,
        b: 47,
    },
    green: Color {
        r: 120,
        g: 138,
        b: 0,
    },
    blue: Color {
        r: 38,
        g: 139,
        b: 210,
    },
    yellow: Color {
        r: 181,
        g: 137,
        b: 0,
    },
    purple: Color {
        r: 108,
        g: 113,
        b: 196,
    },
    cyan: Color {
        r: 39,
        g: 149,
        b: 141,
    },
    pink: Color {
        r: 211,
        g: 54,
        b: 130,
    },
    orange: Color {
        r: 203,
        g: 75,
        b: 22,
    },
    brown: Color {
        r: 115,
        g: 70,
        b: 38,
    },
    grey: Color {
        r: 76,
        g: 95,
        b: 103,
    },
    foreground: Color { r: 0, g: 0, b: 0 },
    background: Color {
        r: 253,
        g: 246,
        b: 227,
    },
};

/// Nord Dark (Arctic Ice Studio / Sven Greb, nordtheme.com/docs/colors-and-
/// palettes; verified 2026-07-13 directly against that page - the
/// THEME_ANALYSIS.md nord0-nord15 table matched with no discrepancies).
/// BACKGROUND is the official Polar Night `nord0` (`#2e3440`); FOREGROUND is
/// the official Snow Storm `nord6` (`#eceff4`). YELLOW and GREEN are the
/// official Aurora `nord13`/`nord14` unchanged - both already clear every
/// gate at their native lightness. CYAN and BLUE are the official Frost
/// `nord8`/`nord9` unchanged, for the same reason.
///
/// RED (`nord11`), ORANGE (`nord12`), PURPLE (`nord15`) are the official
/// Aurora hues, each nudged lighter (hue/saturation unchanged) by
/// `tests::gate_contrast_all_themes`'s `contrast` transform check (4.5:1):
/// at their native lightness none of Nord's warm/magenta Aurora hues reach
/// 4.5:1 against either FOREGROUND or BACKGROUND (unlike the cooler
/// YELLOW/GREEN/CYAN/BLUE, which do) - `#bf616a` (l 56.5%) -> `#cf888f` (l
/// 67.2%); `#d08770` (l 62.7%) -> `#d18a73` (l 63.5%); `#b48ead` (l 63.1%)
/// -> `#b590af` (l 63.8%).
///
/// PINK has no Aurora accent; it is derived at a rose hue between RED
/// (354°) and PURPLE (311°) but pushed saturation to Aurora's upper bound
/// (`nord12`'s 50.5%, vs. a naive midpoint's ~32%) - a straight hue/
/// saturation interpolation between the tuned RED and PURPLE couldn't
/// clear 15.0 deltaE (`gate_contrast_all_themes`'s player-distinctness
/// check) from both at once (best achievable was ~10.2); the extra
/// saturation (hue 315°, s 50%, tuned to l 66.6% for the same 4.5:1
/// transform floor as RED/ORANGE/PURPLE) resolves to `#d47fbf`, clearing
/// both by >20.
///
/// BROWN has no Aurora accent either; it is derived as a desaturated tan
/// near ORANGE's hue (20° vs. `nord12`'s 14.4°, s 19% vs. 50.5%), tuned to
/// `#ae9b89` (l 61%) - chosen (over closer hue/saturation matches to
/// ORANGE) specifically to clear 15.0 deltaE from ORANGE and GREY, both of
/// which sat close by at nearby hues.
///
/// GREY has no Nord slot at a usable lightness: the official comment
/// colour `nord3` (`#4c566a`, l 35.7%) manages only ~1.7:1 against
/// BACKGROUND, far under the 3:1 text floor. GREY is tuned to the same
/// hue family (`nord3`'s ~220°, kept at 230° - a 10° nudge) at a much
/// higher lightness (77%, vs. `nord3`'s 35.7%) and slightly reduced
/// saturation (15% vs. `nord3`'s 16.5%), landing on `#bcbecd` - driven by
/// the text floor against BACKGROUND and every in-use softened surface,
/// and by the 15.0 deltaE floor against BLUE (which sat only 13.1 deltaE
/// away at the first passing lightness/hue).
pub static NORD_DARK: Palette = Palette {
    red: Color {
        r: 207,
        g: 136,
        b: 143,
    },
    green: Color {
        r: 163,
        g: 190,
        b: 140,
    },
    blue: Color {
        r: 129,
        g: 161,
        b: 193,
    },
    yellow: Color {
        r: 235,
        g: 203,
        b: 139,
    },
    purple: Color {
        r: 181,
        g: 144,
        b: 175,
    },
    cyan: Color {
        r: 136,
        g: 192,
        b: 208,
    },
    pink: Color {
        r: 212,
        g: 127,
        b: 191,
    },
    orange: Color {
        r: 209,
        g: 138,
        b: 115,
    },
    brown: Color {
        r: 174,
        g: 155,
        b: 137,
    },
    grey: Color {
        r: 188,
        g: 190,
        b: 205,
    },
    foreground: Color {
        r: 236,
        g: 239,
        b: 244,
    },
    background: Color {
        r: 46,
        g: 52,
        b: 64,
    },
};

/// Nord Light (Arctic Ice Studio / Sven Greb; same source as NORD_DARK,
/// verified 2026-07-13). BACKGROUND is the official Snow Storm `nord6`
/// (`#eceff4`); FOREGROUND is the official Polar Night `nord0` (`#2e3440`),
/// Nord's own dark/light inversion. Shares NORD_DARK's 9 accent hues
/// (RED/ORANGE/YELLOW/GREEN/PURPLE/CYAN/BLUE = `nord11`/`nord12`/`nord13`/
/// `nord14`/`nord15`/`nord8`/`nord9`), each independently tuned - a hue
/// passing on the dark background can fail on the light one and vice
/// versa.
///
/// All 7 official accents needed darkening (hue/saturation unchanged):
/// Nord's Aurora/Frost pastels sit at 52-73% lightness, and none reach
/// 3:1 against the light BACKGROUND (`nord6`, l 94%) at that lightness -
/// this is the "light-mode pastel" case flagged in the task brief. Each
/// was darkened to the minimum lightness clearing both the 3:1 text floor
/// (`contrast_ratio` vs BACKGROUND) and the 4.5:1 `contrast` transform
/// floor: RED `#bf616a` (l 56.5%) -> `#b54954` (l 49.9%); ORANGE
/// `#d08770` (l 62.7%) -> `#aa5338` (l 44.3%); YELLOW `#ebcb8b` (l 73.3%)
/// -> `#8d6618` (l 32.3%); GREEN `#a3be8c` (l 64.7%) -> `#597442` (l
/// 35.7%); PURPLE `#b48ead` (l 63.1%) -> `#8d5d84` (l 45.9%); CYAN
/// `#88c0d0` (l 67.5%) -> `#357587` (l 36.9%); BLUE `#81a1c1` (l 63.1%)
/// -> `#4e6f97` (l 45.0%).
///
/// PINK and BROWN are derived the same way as NORD_DARK (same hue/
/// saturation choices - rose hue 315°/s 50% for PINK, tan hue 20°/s 20%
/// for BROWN - re-tuned in lightness for the light background): PINK ->
/// `#b43c96` (l 47%); BROWN -> `#816456` (l 42%). Both clear the same
/// gates (3:1 text floor, 4.5:1 transform floor, 15.0 deltaE
/// distinctness) that forced NORD_DARK's choice of hue/saturation in the
/// first place.
///
/// GREY is derived the same way as NORD_DARK - `nord3`'s hue family, but
/// darkened rather than lightened for the light background, and nudged
/// less (219°, a 1° drift, vs. `nord3`'s 220°) with slightly reduced
/// saturation (10% vs. `nord3`'s 16.5%) - `#5a606d` (l 39%), driven by the
/// same text-floor/surface/distinctness gates as NORD_DARK's GREY (here
/// BLUE and CYAN sat closest, at 18.1 deltaE at the first passing value).
pub static NORD_LIGHT: Palette = Palette {
    red: Color {
        r: 181,
        g: 73,
        b: 84,
    },
    green: Color {
        r: 89,
        g: 116,
        b: 66,
    },
    blue: Color {
        r: 78,
        g: 111,
        b: 151,
    },
    yellow: Color {
        r: 141,
        g: 102,
        b: 24,
    },
    purple: Color {
        r: 141,
        g: 93,
        b: 132,
    },
    cyan: Color {
        r: 53,
        g: 117,
        b: 135,
    },
    pink: Color {
        r: 180,
        g: 60,
        b: 150,
    },
    orange: Color {
        r: 170,
        g: 83,
        b: 56,
    },
    brown: Color {
        r: 129,
        g: 100,
        b: 86,
    },
    grey: Color {
        r: 90,
        g: 96,
        b: 109,
    },
    foreground: Color {
        r: 46,
        g: 52,
        b: 64,
    },
    background: Color {
        r: 236,
        g: 239,
        b: 244,
    },
};

/// One Dark (Atom's `atom/one-dark-syntax`; verified 2026-07-13 against
/// `styles/colors.less` in that repo, not the THEME_ANALYSIS.md table, which
/// has one error - see below). BACKGROUND (`syntax-bg`, `#282c34`), FOREGROUND
/// (`mono-1`, `#abb2bf`), GREEN (`hue-4`, `#98c379`), BLUE (`hue-2`,
/// `#61afef`), PURPLE (`hue-3`, `#c678dd`) are the official values unchanged.
/// RED is the official `hue-5` (`#e06c75`) lightened for the contrast gate
/// (see below). YELLOW takes the "types" accent (`hue-6-2`,
/// `#e5c07b`) and ORANGE takes the "constants" accent (`hue-6`, `#d19a66`) -
/// One Dark has only one orange-family hue split into two lightnesses, and
/// THEMING.md's RED/ORANGE/YELLOW distinctness gate needs the lighter, more
/// yellow-looking one in the YELLOW slot. CYAN is the official `hue-1`
/// (`#56b6c2`) - THEME_ANALYSIS.md's table omits this hue entirely.
///
/// THEME_ANALYSIS.md's Comment value (`#59626F`) does not match upstream:
/// the actual `mono-3` (comment/punctuation) colour is `#5c6370`. GREY is
/// tuned, not the official value - see below.
///
/// FOREGROUND (official `mono-1`, `#abb2bf`) is lightened for the contrast
/// gate: `contrast(soften(FOREGROUND, 75))` (the softened checkerboard
/// surface contrasted back against itself) only reaches 4.0:1 at the
/// official lightness, under the 4.5:1 transform floor; lightened
/// (hue/saturation unchanged, l 71% -> 78%) to `#bfc5ce`, clearing it at
/// 4.6:1.
///
/// PINK and BROWN have no One Dark accent; both are derived and tuned
/// against `tests::gate_contrast_all_themes`:
/// - PINK sits at a rose hue (328°) between RED (355°) and PURPLE (286°),
///   at RED/PURPLE's own saturation/lightness (65%/66%) -> `#e170ac`.
/// - BROWN is ORANGE's hue (29°) desaturated (30%, vs. ORANGE's 54%),
///   lightened to `#b8987a` (l 60%, vs. an initial 45%) for the 4.5:1
///   `contrast` transform floor (see below).
///
/// Tuning forced by `tests::gate_contrast_all_themes`:
/// - RED (official `#e06c75`, l 65%) reaches only 4.38:1 via `contrast`
///   (which lands on BACKGROUND, the higher-contrast side), under the
///   4.5:1 transform floor; lightened (hue/saturation unchanged) to
///   `#e27881` (l 68%), clearing it at 4.82:1.
/// - BROWN (derived at l 45%, see above) reaches only 3.20:1 via `contrast`
///   (BACKGROUND side), under the 4.5:1 transform floor; lightened
///   (hue/saturation unchanged) to l 60% (`#b8987a`), clearing it at
///   5.21:1.
/// - GREY could not stay on the official comment hue (~220°, shared with
///   FOREGROUND and CYAN's ~191°/hue-1): the official `#5c6370` (l 40%)
///   manages only ~2.3:1 against BACKGROUND, under the 3:1 text floor, and
///   every lightness on that hue clearing the text/transform floors landed
///   within 15.0 deltaE of the (lightened) FOREGROUND or of CYAN. GREY is
///   instead moved to a desaturated hue further from both (100°, s 10%,
///   l 65%) -> `#a3af9d`, clearing BACKGROUND (6.12:1), the softened
///   surface (3.46:1), and >=15.0 deltaE from every player colour,
///   FOREGROUND, and CYAN.
pub static ONE_DARK: Palette = Palette {
    red: Color {
        r: 226,
        g: 120,
        b: 129,
    },
    green: Color {
        r: 152,
        g: 195,
        b: 121,
    },
    blue: Color {
        r: 97,
        g: 175,
        b: 239,
    },
    yellow: Color {
        r: 229,
        g: 192,
        b: 123,
    },
    purple: Color {
        r: 198,
        g: 120,
        b: 221,
    },
    cyan: Color {
        r: 86,
        g: 182,
        b: 194,
    },
    pink: Color {
        r: 225,
        g: 112,
        b: 172,
    },
    orange: Color {
        r: 209,
        g: 154,
        b: 102,
    },
    brown: Color {
        r: 184,
        g: 152,
        b: 122,
    },
    grey: Color {
        r: 163,
        g: 175,
        b: 157,
    },
    foreground: Color {
        r: 191,
        g: 197,
        b: 206,
    },
    background: Color {
        r: 40,
        g: 44,
        b: 52,
    },
};

/// One Light (Atom's `atom/one-light-syntax`; verified 2026-07-13 against
/// `styles/colors.less`, matching THEME_ANALYSIS.md's table with one
/// omission - see below). BACKGROUND (`syntax-bg`, `#fafafa`), FOREGROUND
/// (`mono-1`, `#383a42`), PURPLE (`hue-3`, `#a626a4`) are the official values
/// unchanged. ORANGE takes the "constants" accent (`hue-6`, `#986801`,
/// unchanged); YELLOW takes the "types" accent (`hue-6-2`) but is retuned
/// (see below) rather than kept at its official `#c18401` - upstream's
/// hue-6/hue-6-2 share the exact same hue (41°), differing only in
/// lightness, which fails THEMING.md's RED/ORANGE/YELLOW distinctness gate
/// outright. CYAN is the official `hue-1` (`#0184bc`, nudged - see below) -
/// THEME_ANALYSIS.md's table omits this hue entirely (the only value it
/// lists that doesn't match upstream is missing altogether; every value it
/// does list matched exactly, including Comment `#a0a1a7` = `mono-3`).
///
/// PINK and BROWN have no One Light accent; both are derived the same way as
/// ONE_DARK, re-tuned for a light background:
/// - PINK is the same rose hue (328°) between RED and PURPLE, at a
///   lightness/saturation suited to text on a light background (63%/45%)
///   -> `#bb2a78`.
/// - BROWN is ORANGE's hue (29°), desaturated and darkened further than
///   ONE_DARK's (45%/30%, vs. ORANGE's 99%/30%) -> `#6f4b2a`.
///
/// Tuning forced by `tests::gate_contrast_all_themes` (unlike ONE_DARK,
/// every hue here needed a change, not just GREY/BROWN/RED - light
/// backgrounds need darker accents than upstream's mid-tone syntax colours
/// to clear the 4.5:1 `contrast` floor):
/// - RED (official `#e45649`, l 59%) reaches only 3.51:1 via `contrast`;
///   darkened to l 46% (`#cc2d1e`), clearing it at 5.08:1.
/// - GREEN (official `#50a14f`, l 47%) reaches only 3.54:1; darkened to
///   l 34% (`#3e7b3d`), clearing it at 5.38:1.
/// - BLUE (official `#4078f2`, l 60%) reaches only 3.88:1; darkened to
///   l 52% (`#1a5eef`), clearing it at 5.17:1.
/// - YELLOW cannot stay on ORANGE's hue (41°, see above); moved to hue 49°
///   at l 41% (`#d0aa01`), clearing both the 4.5:1 floor (5.09:1) and
///   15.0 deltaE from ORANGE and RED.
/// - CYAN (official `#0184bc`, l 37%) reaches only 4.00:1; darkened to
///   l 34% (`#0179ad`), clearing it at 4.64:1.
/// - GREY (official comment colour, `mono-3`, `#a0a1a7`, l 64%) manages only
///   ~2.0:1 against BACKGROUND, under the 3:1 text floor; darkened
///   (hue/saturation unchanged) to `#6c6e7a` (l 45%), clearing the floor
///   and 15.0 deltaE from BROWN.
pub static ONE_LIGHT: Palette = Palette {
    red: Color {
        r: 204,
        g: 45,
        b: 30,
    },
    green: Color {
        r: 62,
        g: 123,
        b: 61,
    },
    blue: Color {
        r: 26,
        g: 94,
        b: 239,
    },
    yellow: Color {
        r: 208,
        g: 170,
        b: 1,
    },
    purple: Color {
        r: 166,
        g: 38,
        b: 164,
    },
    cyan: Color {
        r: 1,
        g: 121,
        b: 173,
    },
    pink: Color {
        r: 187,
        g: 42,
        b: 120,
    },
    orange: Color {
        r: 152,
        g: 104,
        b: 1,
    },
    brown: Color {
        r: 111,
        g: 75,
        b: 42,
    },
    grey: Color {
        r: 108,
        g: 110,
        b: 122,
    },
    foreground: Color {
        r: 56,
        g: 58,
        b: 66,
    },
    background: Color {
        r: 250,
        g: 250,
        b: 250,
    },
};

/// Gruvbox Dark (Pavel Pertsev / morhetz, github.com/morhetz/gruvbox;
/// verified 2026-07-13 against `colors/gruvbox.vim` - THEME_ANALYSIS.md's
/// table matched with no discrepancies: `bright_red`/`bright_green`/
/// `bright_yellow`/`bright_blue`/`bright_purple`/`bright_aqua`/
/// `bright_orange`/`gray`/`bg0`/`fg1` all confirmed byte-identical). BACKGROUND
/// is the official medium-contrast `bg0` (`#282828`). GREEN, YELLOW, ORANGE
/// are the official `bright_green`/`bright_yellow`/`bright_orange` values
/// unchanged. CYAN takes `bright_aqua` (`#8ec07c`) - Gruvbox's "aqua" reads
/// more green than cyan, but it's the closest official hue to the slot and
/// stays well clear of GREEN (aqua is desaturated and cooler). BLUE takes the
/// official `bright_blue` (`#83a598`), a muted teal-blue; kept deliberately
/// distinct from CYAN per THEMING.md's BLUE/CYAN gate. PURPLE takes the
/// official `bright_purple` (`#d3869b`), which Gruvbox itself renders as a
/// dusty rose rather than a violet.
///
/// PINK and BROWN have no Gruvbox accent (its 8-colour terminal set has no
/// rose/brown split beyond the ones already used above) and are derived:
/// - PINK is a magenta hue (315°) distinct from both RED (~6°) and the
///   rose-toned PURPLE (~344°), at high saturation (65%) and a lightness
///   (65%) picked for the 4.5:1 `contrast` transform floor -> `#e06cc3`.
/// - BROWN is `bright_orange`'s hue (~27°) desaturated (~40%) and darkened
///   to l 67% -> `#d28246`, clearing 15.0 deltaE from GREY and ORANGE and
///   the 4.5:1 `contrast` floor.
///
/// Tuning forced by `tests::gate_contrast_all_themes`:
/// - RED (official `bright_red` `#fb4934`, l 59%) reaches only 4.29:1 via
///   `contrast` (BACKGROUND side, under the 4.5:1 transform floor);
///   lightened (hue/saturation unchanged) to l 61% (`#ff553c`), clearing it
///   at 4.65:1.
/// - GREY: the official `gray` (`#928374`, l 51%) manages only 2.11:1
///   against `soften(FOREGROUND, 86)`, under the 3:1 text floor (this
///   background is much darker than Gruvbox's own bg0/bg1, so the official
///   mid-lightness gray reads as too dark here). Moved to the same hue
///   family lightened substantially (l 70%, s reduced to 12% from the
///   official ~12% at a cooler, less orange-leaning point on the ramp) ->
///   `#bcbca9`, clearing both softened surfaces and 15.0 deltaE from the
///   (also-adjusted) FOREGROUND.
/// - FOREGROUND: official `fg1` (`#ebdbb2`) only reaches 3.91:1 via
///   `contrast(soften(FOREGROUND, 75))`, under the 4.5:1 transform floor.
///   Desaturating (s 59% -> 25%) and lightening slightly (l 81% -> 90%,
///   hue unchanged) to `#ece8df` clears it at 5.09:1 - the softened
///   surface pulls less toward orange at lower saturation, which is what
///   unblocks the floor.
/// - PINK/BROWN (see derivation above) both needed their initial guesses
///   pushed further from GREY's lightness to clear 4.5:1 via `contrast`.
pub static GRUVBOX_DARK: Palette = Palette {
    red: Color {
        r: 255,
        g: 85,
        b: 60,
    },
    green: Color {
        r: 184,
        g: 187,
        b: 38,
    },
    blue: Color {
        r: 131,
        g: 165,
        b: 152,
    },
    yellow: Color {
        r: 250,
        g: 189,
        b: 47,
    },
    purple: Color {
        r: 211,
        g: 134,
        b: 155,
    },
    cyan: Color {
        r: 142,
        g: 192,
        b: 124,
    },
    pink: Color {
        r: 224,
        g: 108,
        b: 195,
    },
    orange: Color {
        r: 254,
        g: 128,
        b: 25,
    },
    brown: Color {
        r: 210,
        g: 130,
        b: 70,
    },
    grey: Color {
        r: 188,
        g: 188,
        b: 169,
    },
    foreground: Color {
        r: 236,
        g: 232,
        b: 223,
    },
    background: Color {
        r: 40,
        g: 40,
        b: 40,
    },
};

/// Gruvbox Light (same source as GRUVBOX_DARK, verified 2026-07-13).
/// BACKGROUND is the official medium-contrast `bg0` (`#fbf1c7`); FOREGROUND
/// is the official `fg1` (`#3c3836`). BLUE and PURPLE are the official
/// "faded" accents (`faded_blue` `#076678`, `faded_purple` `#8f3f71`)
/// unchanged - Gruvbox's own light-mode set, already darkened for a light
/// background.
///
/// PINK and BROWN have no Gruvbox accent, derived the same way as
/// GRUVBOX_DARK, re-tuned for a light background:
/// - PINK is the same magenta hue (315°) as GRUVBOX_DARK, darkened/
///   resaturated for text on light (s 80%, l 37%) -> `#a11a72`.
/// - BROWN is `faded_orange`'s hue (~16°) desaturated (~40%) and lightened
///   slightly above `faded_orange`'s own lightness (l 35%) -> `#874a2b`,
///   clearing 15.0 deltaE from both GREY and ORANGE.
///
/// Tuning forced by `tests::gate_contrast_all_themes` (every official
/// accent needed darkening for the light background, and GREY needed
/// tuning - the same "light-mode pastel" pattern seen in NORD_LIGHT/
/// ONE_LIGHT):
/// - RED (official `faded_red` `#9d0006`, l 31%) is unchanged - it already
///   clears every floor at its official value.
/// - GREEN (official `faded_green` `#79740e`, l 27%) reaches only 4.29:1
///   via `contrast`; darkened (hue/saturation unchanged) to l 24%
///   (`#6e690c`), clearing it at 5.02:1.
/// - YELLOW (official `faded_yellow` `#b57614`, l 39%) reaches only 3.33:1;
///   darkened to l 31% (`#915e0f`), clearing it at 4.86:1.
/// - CYAN (official `faded_aqua` `#427b58`, l 37%) reaches only 4.40:1;
///   darkened to l 33% (`#3a6c4d`), clearing it at 5.39:1.
/// - ORANGE (official `faded_orange` `#af3a03`, l 35%) is unchanged - it
///   already clears every floor at its official value.
/// - GREY: the official `gray` (`#928374`, l 51%) manages only 2.23:1
///   against `soften(FOREGROUND, 86)`, under the 3:1 text floor (this
///   background is much lighter than Gruvbox's own bg0/bg1 pairing, so the
///   official mid-lightness gray reads as too light here). Darkened on the
///   same warm hue family to l 35% (s 12%) -> `#64594f`, clearing both
///   softened surfaces and 15.0 deltaE from FOREGROUND.
pub static GRUVBOX_LIGHT: Palette = Palette {
    red: Color { r: 157, g: 0, b: 6 },
    green: Color {
        r: 110,
        g: 105,
        b: 12,
    },
    blue: Color {
        r: 7,
        g: 102,
        b: 120,
    },
    yellow: Color {
        r: 145,
        g: 94,
        b: 15,
    },
    purple: Color {
        r: 143,
        g: 63,
        b: 113,
    },
    cyan: Color {
        r: 58,
        g: 108,
        b: 77,
    },
    pink: Color {
        r: 161,
        g: 26,
        b: 114,
    },
    orange: Color {
        r: 175,
        g: 58,
        b: 3,
    },
    brown: Color {
        r: 135,
        g: 74,
        b: 43,
    },
    grey: Color {
        r: 100,
        g: 89,
        b: 79,
    },
    foreground: Color {
        r: 60,
        g: 56,
        b: 54,
    },
    background: Color {
        r: 251,
        g: 241,
        b: 199,
    },
};

/// Catppuccin Mocha (github.com/catppuccin/catppuccin; verified 2026-07-13
/// against the README palette table - THEME_ANALYSIS.md's Mocha table
/// matched with no discrepancies). BACKGROUND is the official `base`
/// (`#1e1e2e`); FOREGROUND is the official `text` (`#cdd6f4`). RED, GREEN,
/// YELLOW, PINK are the official `red`/`green`/`yellow`/`pink` values
/// unchanged - all already clear every gate at native lightness. PURPLE
/// takes the official `mauve` (`#cba6f7`); ORANGE takes the official
/// `peach` (`#fab387`).
///
/// CYAN takes the official `sky` (`#89dceb`) rather than `teal` or
/// `sapphire`: `sky` reads the most cyan of the three, and its hue (~189°)
/// sits far enough from `blue`'s (~217°) to clear THEMING.md's BLUE/CYAN
/// distinctness gate, unlike `sapphire` (~199°) which sat too close.
///
/// GREY is tuned, not an official Catppuccin value: neither `overlay0`
/// (`#6c7086`, l 47%) nor `overlay1` (`#7f849c`, l 55%) clears the 3:1 text
/// floor against `soften(FOREGROUND, 75)` (2.87:1 at `overlay1`) - forced by
/// `tests::gate_contrast_all_themes`. GREY is lightened on `overlay2`'s hue
/// family (228°) to l 64% (`#9399b2`, in fact `overlay2` itself), clearing
/// the floor at 3.3:1+ and 15.0 deltaE from BLUE and FOREGROUND.
///
/// BROWN has no Catppuccin accent; it is derived at `peach`'s hue (~23°)
/// desaturated and darkened to read as a tan/brown rather than an orange
/// wash (`#ab8060`, s 30%/l 51%, vs. peach's s 92%/l 75%), chosen to clear
/// 15.0 deltaE from ORANGE and GREY.
///
/// No tuning was forced by `tests::gate_contrast_all_themes` beyond the
/// GREY/BROWN choices above - every other official value clears the gate at
/// its native value.
pub static CATPPUCCIN_MOCHA: Palette = Palette {
    red: Color {
        r: 243,
        g: 139,
        b: 168,
    },
    green: Color {
        r: 166,
        g: 227,
        b: 161,
    },
    blue: Color {
        r: 137,
        g: 180,
        b: 250,
    },
    yellow: Color {
        r: 249,
        g: 226,
        b: 175,
    },
    purple: Color {
        r: 203,
        g: 166,
        b: 247,
    },
    cyan: Color {
        r: 137,
        g: 220,
        b: 235,
    },
    pink: Color {
        r: 245,
        g: 194,
        b: 231,
    },
    orange: Color {
        r: 250,
        g: 179,
        b: 135,
    },
    brown: Color {
        r: 171,
        g: 128,
        b: 96,
    },
    grey: Color {
        r: 166,
        g: 172,
        b: 194,
    },
    foreground: Color {
        r: 205,
        g: 214,
        b: 244,
    },
    background: Color {
        r: 30,
        g: 30,
        b: 46,
    },
};

/// Catppuccin Latte (github.com/catppuccin/catppuccin; same source as
/// CATPPUCCIN_MOCHA, verified 2026-07-13 - THEME_ANALYSIS.md's Latte table
/// matched with no discrepancies). BACKGROUND is the official `base`
/// (`#eff1f5`); FOREGROUND is the official `text` (`#4c4f69`). PURPLE takes
/// the official `mauve` (`#8839ef`) unchanged - it already clears every
/// floor at its native value.
///
/// CYAN takes `sky` (same choice as CATPPUCCIN_MOCHA, for the same
/// BLUE/CYAN distinctness reason), but - like RED/GREEN/YELLOW/BLUE/
/// ORANGE/PINK below - Latte's official light-mode accents are all too
/// light to clear THEMING.md's floors against this light BACKGROUND
/// (l 95%), the same "light-mode pastel" pattern seen in NORD_LIGHT/
/// ONE_LIGHT/GRUVBOX_LIGHT; every hue below is the official value darkened
/// (hue/saturation unchanged) to the point clearing both the 3:1 text floor
/// and the 4.5:1 `contrast` transform floor, per
/// `tests::gate_contrast_all_themes`:
/// - RED (official `#d20f39`, l 44%) is unchanged - already clears every
///   floor.
/// - GREEN (official `#40a02b`, l 40%) reaches only 2.96:1 against
///   BACKGROUND (under the 3:1 text floor); darkened to l 33%
///   (`#278521`), clearing it at 3.86:1 (and 4.5:1+ via `contrast`).
/// - YELLOW (official `#df8e1d`, l 49%) reaches only 3.99:1 via `contrast`;
///   darkened to l 28% (`#7e5110`), clearing it at 5.09:1.
/// - BLUE (official `#1e66f5`, l 54%) reaches only 4.10:1 via `contrast`;
///   darkened to l 47% (`#1a58d6`), clearing it at 4.79:1.
/// - CYAN (official `sky` `#04a5e5`, l 46%) reaches only 2.47:1 against
///   BACKGROUND (under the 3:1 text floor); darkened to l 28% (`#02658d`),
///   clearing it at 4.67:1 via `contrast`.
/// - ORANGE (official `peach` `#fe640b`, l 52%) reaches only 2.68:1 via
///   `contrast`; darkened to l 34% (`#ad4001`), clearing it at 5.14:1.
/// - PINK (official `#ea76cb`, l 69%) reaches only 2.34:1 against
///   BACKGROUND; darkened to l 42% (`#b91d90`), clearing it at 3.06:1 (and
///   4.5:1+ via `contrast`).
///
/// BROWN has no Catppuccin accent; it is derived at `peach`'s hue (~22°)
/// desaturated and darkened, the same approach as CATPPUCCIN_MOCHA
/// (`#7a4f34`, s 40%/l 34%), chosen to clear 15.0 deltaE from ORANGE and
/// GREY.
///
/// GREY is tuned, not an official value: the official `overlay0`
/// (`#9ca0b0`, l 65%) manages only 1.86:1 against BACKGROUND, far under the
/// 3:1 text floor (Latte's overlay ramp is built for text *on*
/// `base`/`mantle`/`crust`, all close in lightness to `overlay0` itself,
/// not for a similarly-light theme BACKGROUND at l 95%). A first attempt
/// lightened along the same hue family failed a different floor: it needs
/// to stay >=15.0 deltaE from the (now much darker) FOREGROUND while also
/// clearing >=3:1 against `soften(FOREGROUND, 75)` (a very light surface at
/// this lightness pairing), which no lightness on that hue cleared at
/// once. GREY is instead moved to a near-neutral slate (`hue 200°, s 5%`)
/// and darkened to l 24% (`#3a3e40`), clearing both floors and 17.2 deltaE
/// from FOREGROUND (vs. the 15.0 floor).
pub static CATPPUCCIN_LATTE: Palette = Palette {
    red: Color {
        r: 210,
        g: 15,
        b: 57,
    },
    green: Color {
        r: 39,
        g: 97,
        b: 26,
    },
    blue: Color {
        r: 26,
        g: 88,
        b: 214,
    },
    yellow: Color {
        r: 126,
        g: 81,
        b: 16,
    },
    purple: Color {
        r: 136,
        g: 57,
        b: 239,
    },
    cyan: Color {
        r: 2,
        g: 101,
        b: 141,
    },
    pink: Color {
        r: 185,
        g: 29,
        b: 144,
    },
    orange: Color {
        r: 173,
        g: 64,
        b: 1,
    },
    brown: Color {
        r: 122,
        g: 79,
        b: 52,
    },
    grey: Color {
        r: 58,
        g: 62,
        b: 64,
    },
    foreground: Color {
        r: 76,
        g: 79,
        b: 105,
    },
    background: Color {
        r: 239,
        g: 241,
        b: 245,
    },
};

/// Tokyo Night (github.com/enkia/tokyo-night-vscode-theme, cross-checked
/// against folke/tokyonight.nvim's `colors/storm.lua`; verified 2026-07-13).
/// RED (`#f7768e`), GREEN (`#9ece6a`), BLUE (`#7aa2f7`), YELLOW (`#e0af68`),
/// PURPLE (`#bb9af7`, upstream's "magenta"), CYAN (`#7dcfff`), ORANGE
/// (`#ff9e64`) are the official values, unchanged - all clear every gate at
/// native lightness. FOREGROUND is the official `fg_dark`/editor-foreground
/// value (`#a9b1d6`) - the vscode theme's `editor.foreground`, distinct from
/// the brighter `fg` (`#c0caf5`) the nvim source also defines but the vscode
/// theme never surfaces as body text. BACKGROUND is the official `bg`
/// (`#1a1b26`).
///
/// GREY is not the official Comment colour: the two upstreams disagree
/// (`tokyonight.nvim`'s `colors.comment` is `#565f89`; the vscode theme's own
/// "Comment" token rule is `#5f6996` instead) and neither clears the 3:1 text
/// floor against the softened checkerboard surfaces at this dark a
/// BACKGROUND (`#565f89` reaches only ~1.85:1 against
/// `soften(FOREGROUND, 75)`) or stays >=15.0 deltaE from FOREGROUND once
/// lightened enough to clear it (every lightness on that hue (~229°) that
/// clears the surface floor lands within 15.0 deltaE of FOREGROUND, the same
/// "hue shared with FOREGROUND" bind seen in CATPPUCCIN_MOCHA). GREY is
/// moved to a cooler, desaturated hue (200°, s 25%) at l 74% -> `#acc2cd`,
/// clearing every softened-surface floor and >=15.0 deltaE from FOREGROUND
/// and BACKGROUND.
///
/// PINK and BROWN have no accent in either upstream source (Tokyo Night's
/// "magenta2"/`#ff007c` exists in the nvim palette table but is never
/// surfaced in the vscode extension's token colours, and isn't used here
/// because it fails `tests::gate_contrast_all_themes`'s PINK/RED/PURPLE
/// distinctness at any lightness that also clears the text and transform
/// floors simultaneously - see below). Both are derived:
/// - PINK is an orchid hue (290°, s 60%) distinct from RED (~349°) and
///   PURPLE/magenta (~267°), tuned to l 65% (`#c970db`) for the contrast
///   gates below.
/// - BROWN is ORANGE's hue (~22°) desaturated (s 25%, vs ORANGE's 100%) and
///   darkened to l 52% (`#a39266`), clearing 15.0 deltaE from ORANGE and
///   GREY.
///
/// Tuning forced by `tests::gate_contrast_all_themes` (beyond GREY/PINK/
/// BROWN's derivation above, which already bakes in the gate-driven choices):
/// PINK could not use the nvim source's `magenta2` (`#ff007c`, l 50%):
/// `contrast(soften(PINK, 75))` (the pale wash contrasted back against
/// itself) reaches only 4.05:1 against the 4.5:1 transform floor at that
/// lightness, and no lightness on that hue (330°) clears both this floor and
/// PINK's own `contrast` floor at once (lightening improves one and worsens
/// the other - the same trade-off pattern as other derived accents). Moving
/// to hue 290°/s 60% and l 65% (`#c970db`) clears both simultaneously.
pub static TOKYO_NIGHT: Palette = Palette {
    red: Color {
        r: 247,
        g: 118,
        b: 142,
    },
    green: Color {
        r: 158,
        g: 206,
        b: 106,
    },
    blue: Color {
        r: 122,
        g: 162,
        b: 247,
    },
    yellow: Color {
        r: 224,
        g: 175,
        b: 104,
    },
    purple: Color {
        r: 187,
        g: 154,
        b: 247,
    },
    cyan: Color {
        r: 125,
        g: 207,
        b: 255,
    },
    pink: Color {
        r: 201,
        g: 112,
        b: 219,
    },
    orange: Color {
        r: 255,
        g: 158,
        b: 100,
    },
    brown: Color {
        r: 163,
        g: 146,
        b: 102,
    },
    grey: Color {
        r: 172,
        g: 194,
        b: 205,
    },
    foreground: Color {
        r: 169,
        g: 177,
        b: 214,
    },
    background: Color {
        r: 26,
        g: 27,
        b: 38,
    },
};

/// Tokyo Night Storm (same source and accents as TOKYO_NIGHT; verified
/// 2026-07-13). Only BACKGROUND differs from TOKYO_NIGHT - the official
/// `bg` for the Storm variant (`#24283b`, vs Night's `#1a1b26`) - per
/// upstream (both `tokyonight.nvim` and the vscode extension use the same
/// accent set and FOREGROUND across Night/Storm, differing only in
/// BACKGROUND).
///
/// Every accent is independently re-checked against this lighter BACKGROUND
/// (a hue passing on one background can fail on the other): RED/GREEN/BLUE/
/// YELLOW/PURPLE/CYAN/ORANGE/GREY/BROWN all still clear every floor unchanged
/// from TOKYO_NIGHT.
///
/// PINK needed the same re-derivation as TOKYO_NIGHT (not an additional
/// Storm-specific tune - the derived `#c970db` was chosen to clear both
/// Night's and Storm's BACKGROUNDs at once, since the two variants share a
/// single PINK value): the official `magenta2` (`#ff007c`) fails Storm's
/// `contrast(soften(PINK, 75))` even more than Night's (Storm's lighter
/// BACKGROUND pulls the softened wash closer to washed-out), forcing the
/// hue/lightness move described on TOKYO_NIGHT.
pub static TOKYO_NIGHT_STORM: Palette = Palette {
    background: Color {
        r: 36,
        g: 40,
        b: 59,
    },
    ..TOKYO_NIGHT
};

/// Tokyo Night Light (github.com/enkia/tokyo-night-vscode-theme's
/// `tokyo-night-light-color-theme.json`; verified 2026-07-13). RED
/// (`#8c4351`), ORANGE (`#965027`), YELLOW (`#8f5e15`), GREEN (`#385f0d`),
/// BLUE (`#2959aa`), PURPLE (`#5a3e8e`, "storage tags"/magenta) are the
/// official semantic-token values unchanged - all clear every gate natively
/// (light-mode themes usually need darkening, per NORD_LIGHT/ONE_LIGHT/
/// GRUVBOX_LIGHT/CATPPUCCIN_LATTE, but Tokyo Night Light's accents are
/// already dark syntax-on-light-canvas colours). BACKGROUND is the official
/// `editor.background` (`#e6e7ed`); FOREGROUND is the official
/// `variable`/`editor.foreground` (`#343b58`; the file's literal
/// `editor.foreground` is `#343b59`, one unit off in the blue channel from
/// the semantic-token `variable` value also called out as
/// "Editor Foreground" in THEME_ANALYSIS.md's table - both round to the same
/// colour, and `#343b58` is used here to match the semantic-token source).
///
/// CYAN is the official `#006c86` ("language support functions"), not
/// `#0f4b6e` ("object properties" - THEME_ANALYSIS.md's suggested CYAN):
/// `#0f4b6e` sits only 13.2 deltaE from FOREGROUND (`#343b58`, itself a dark
/// blue-hued near-navy), under `tests::gate_contrast_all_themes`'s 15.0
/// floor; `#006c86`'s more teal hue clears it at 27.4 deltaE while remaining
/// one of upstream's two candidate "cyan" tokens.
///
/// GREY is not the official Comment colour: `#6c6e75` (l 44%) manages only
/// ~2.9:1 against the strongest softened surfaces (`soften(FOREGROUND, 75)`
/// etc.), under the 3:1 text floor - forced by
/// `tests::gate_contrast_all_themes`. Darkened on the same near-neutral hue
/// (227°, s 4%) to l 35% (`#56575d`), clearing every softened surface at
/// >=3:1.
///
/// PINK and BROWN have no accent in the upstream light theme; both are
/// derived the same way as TOKYO_NIGHT/TOKYO_NIGHT_STORM, re-tuned for a
/// light background:
/// - PINK is the same orchid hue (290°) as the dark variants, resaturated
///   and darkened for text on light (s 85%, l 43%) -> `#ac10cb`, clearing
///   15.0 deltaE from RED and PURPLE and every contrast floor.
/// - BROWN is ORANGE's hue (~15°, nudged slightly warmer than ORANGE's own
///   22° to sit further from it) desaturated (s 50%) and darkened well below
///   ORANGE's lightness (l 20%) -> `#4d2619`, clearing 15.0 deltaE from
///   ORANGE and GREY and the 4.5:1 `contrast` floor.
pub static TOKYO_NIGHT_LIGHT: Palette = Palette {
    red: Color {
        r: 140,
        g: 67,
        b: 81,
    },
    green: Color {
        r: 56,
        g: 95,
        b: 13,
    },
    blue: Color {
        r: 41,
        g: 89,
        b: 170,
    },
    yellow: Color {
        r: 143,
        g: 94,
        b: 21,
    },
    purple: Color {
        r: 90,
        g: 62,
        b: 142,
    },
    cyan: Color {
        r: 0,
        g: 108,
        b: 134,
    },
    pink: Color {
        r: 172,
        g: 16,
        b: 203,
    },
    orange: Color {
        r: 150,
        g: 80,
        b: 39,
    },
    brown: Color {
        r: 77,
        g: 38,
        b: 25,
    },
    grey: Color {
        r: 86,
        g: 87,
        b: 93,
    },
    foreground: Color {
        r: 52,
        g: 59,
        b: 88,
    },
    background: Color {
        r: 230,
        g: 231,
        b: 237,
    },
};

/// Night Owl (Sarah Drasner, github.com/sdras/night-owl-vscode-theme;
/// verified 2026-07-13 against `themes/Night Owl-color-theme.json` directly -
/// THEME_ANALYSIS.md's table has several errors, noted below). BACKGROUND
/// (`editor.background`, `#011627`) and FOREGROUND (`editor.foreground`,
/// `#d6deeb`) are the official values, unchanged. GREY is the official
/// Comment colour (`#637777`), unchanged. RED is the official
/// `editorError.foreground` (`#ef5350`) - THEME_ANALYSIS.md's table lists
/// this correctly. PURPLE is the official Keyword colour (`#c792ea`),
/// unchanged. CYAN is the official `keyword.operator` colour (`#7fdbca`,
/// upstream's "Built-in Support" per THEME_ANALYSIS.md, and confirmed here),
/// unchanged. ORANGE is the official `constant.numeric` colour (`#f78c6c`),
/// unchanged. BLUE is the official "Library (function & constant)" colour
/// (`support.function`/`support.constant`, `#82aaff`) - THEME_ANALYSIS.md
/// correctly lists this as "Functions/Methods".
///
/// GREEN takes the official "Library class/type" colour (`support.type`/
/// `support.class`, `#c5e478`) - THEME_ANALYSIS.md's table omits this token
/// entirely (it lists Built-in Support as `#7FDBCA`, which is actually the
/// `keyword.operator` colour used above for CYAN). YELLOW takes the official
/// "Class name" colour (`entity.name.class`, `#ffcb8b`) - also missing from
/// THEME_ANALYSIS.md's table.
///
/// PINK has no single canonical upstream token; it takes the official
/// `invalid` marker colour (`#ff2c83`), a hot magenta distinct in hue (~332°)
/// from both RED (~2°) and the `constant.language.boolean` colour
/// (`#ff5874`, ~350°) that sits too close to RED to use here.
///
/// BROWN has no Night Owl accent; it is derived at ORANGE's hue (~14°)
/// desaturated and darkened to read as a tan/brown rather than an orange
/// wash, the same approach used for the other ported themes.
///
/// Tuning forced by `tests::gate_contrast_all_themes`: GREY (official
/// Comment, `#637777`, l 40%) reaches only 2.93:1 against
/// `soften(FOREGROUND, 86)`, under the 3:1 text floor; lightened
/// (hue/saturation unchanged) to l 56% (`#829c9c`), clearing it at 3.3:1+.
/// Every other official value clears every floor at its native lightness.
pub static NIGHT_OWL: Palette = Palette {
    red: Color {
        r: 239,
        g: 83,
        b: 80,
    },
    green: Color {
        r: 197,
        g: 228,
        b: 120,
    },
    blue: Color {
        r: 130,
        g: 170,
        b: 255,
    },
    yellow: Color {
        r: 255,
        g: 203,
        b: 139,
    },
    purple: Color {
        r: 199,
        g: 146,
        b: 234,
    },
    cyan: Color {
        r: 127,
        g: 219,
        b: 202,
    },
    pink: Color {
        r: 255,
        g: 44,
        b: 131,
    },
    orange: Color {
        r: 247,
        g: 140,
        b: 108,
    },
    brown: Color {
        r: 168,
        g: 122,
        b: 92,
    },
    grey: Color {
        r: 130,
        g: 156,
        b: 156,
    },
    foreground: Color {
        r: 214,
        g: 222,
        b: 235,
    },
    background: Color { r: 1, g: 22, b: 39 },
};

/// Light Owl (Sarah Drasner's official light variant, distributed as "Night
/// Owl Light" in github.com/sdras/night-owl-vscode-theme's
/// `themes/Night Owl-Light-color-theme.json`; verified 2026-07-13).
/// BACKGROUND (`editor.background`, `#fbfbfb`) and FOREGROUND
/// (`editor.foreground`, `#403f53`) are the official values, unchanged. GREY
/// is the official Comment colour (`#989fb1`) - THEME_ANALYSIS.md's guess of
/// `#93A1A1` was wrong; that hex is actually `focusBorder`, an unrelated UI
/// colour, and does not appear as any syntax token in this theme.
///
/// RED is the official `editorError.foreground` (`#e64d49`) -
/// THEME_ANALYSIS.md's suggested `#D32F2F`/`#DE3D3B` do not appear anywhere
/// in the upstream file. PURPLE is the official Keyword colour (`#994cc3`),
/// confirming THEME_ANALYSIS.md's `#9946B2` was a close-but-wrong guess (the
/// real value differs in the last byte and green channel). CYAN is the
/// official Built-in colour (`support.constant.meta.property-value`,
/// `#0c969b`), matching THEME_ANALYSIS.md exactly. BLUE is the official
/// colour shared by `string` (bare), `variable`, and most support tokens
/// (`#4876d6`), matching THEME_ANALYSIS.md's Functions value, though upstream
/// uses it far more broadly than just functions.
///
/// PINK takes the official `constant.numeric` colour (`#aa0982`), a
/// saturated magenta - THEME_ANALYSIS.md's guess that this token holds
/// `#AA5D00` (an amber) is wrong; that hex does not appear in the upstream
/// file at all, and the real Number colour reads as pink/magenta (hue
/// ~322°), not orange. THEME_ANALYSIS.md's suggested string-literal colour
/// (`#49A06F`, green) is also wrong: upstream's `string.quoted` token is
/// `#c96765`, a salmon-red too close in hue (~4°) to RED (~4°) to use as a
/// distinct slot, and is not used here.
///
/// Light Owl's official palette covers only 5 accent hues (red, purple,
/// cyan, blue, and the magenta used above for PINK) - unlike NIGHT_OWL,
/// there is no official green, yellow, orange, or brown token anywhere in
/// the upstream file. GREEN, YELLOW, ORANGE, and BROWN are all derived,
/// spaced through the red-to-cyan hue range NIGHT_OWL's official
/// green/yellow/orange occupy, at lightness/saturation suited to text on a
/// light background: ORANGE at hue 30° (`#a65d00`), YELLOW at hue 43°
/// (`#8c6a12`), GREEN at hue 91° (`#4b7a1f`), BROWN at ORANGE's hue
/// desaturated and darkened further (`#7a5240`) - all four passed the gate
/// as originally derived.
///
/// Tuning forced by `tests::gate_contrast_all_themes`:
/// - RED (official `#e64d49`, l 59%) reaches only 3.67:1 via `contrast`
///   (the 4.5:1 transform floor); darkened (hue/saturation unchanged) to
///   l 40% (`#b31d19`), clearing it at 5.2:1+.
/// - BLUE (official `#4876d6`, l 56%) reaches only 4.19:1 via `contrast`;
///   darkened to l 42% (`#2753af`), clearing it at 4.8:1+.
/// - CYAN (derived-adjacent official `#0c969b`, l 32%) reaches only 3.47:1
///   via `contrast`; darkened to l 28% (`#0a7f83`), clearing it at 4.6:1+.
/// - GREY: no near-neutral tone on FOREGROUND's own blue-violet hue clears
///   both the 3:1 text floor against the softened surfaces and 15.0 deltaE
///   from FOREGROUND at once (every lightness that cleared the surfaces
///   landed within 15.0 deltaE of FOREGROUND, the same "hue shared with
///   FOREGROUND" bind seen in CATPPUCCIN_MOCHA/TOKYO_NIGHT). GREY is instead
///   moved to a warm near-neutral hue (40°, s 8%) at l 30% -> `#534f46`,
///   clearing every softened surface and 19.1 deltaE from FOREGROUND (and
///   19.2 from the derived BROWN).
pub static LIGHT_OWL: Palette = Palette {
    red: Color {
        r: 179,
        g: 29,
        b: 25,
    },
    green: Color {
        r: 75,
        g: 122,
        b: 31,
    },
    blue: Color {
        r: 39,
        g: 83,
        b: 175,
    },
    yellow: Color {
        r: 140,
        g: 106,
        b: 18,
    },
    purple: Color {
        r: 153,
        g: 76,
        b: 195,
    },
    cyan: Color {
        r: 10,
        g: 127,
        b: 131,
    },
    pink: Color {
        r: 170,
        g: 9,
        b: 130,
    },
    orange: Color {
        r: 166,
        g: 93,
        b: 0,
    },
    brown: Color {
        r: 122,
        g: 82,
        b: 64,
    },
    grey: Color {
        r: 83,
        g: 79,
        b: 70,
    },
    foreground: Color {
        r: 64,
        g: 63,
        b: 83,
    },
    background: Color {
        r: 251,
        g: 251,
        b: 251,
    },
};

/// SynthWave '84 (Robb Owen, github.com/robb0wen/synthwave-vscode; verified
/// 2026-07-13 against `themes/synthwave-color-theme.json` directly -
/// THEME_ANALYSIS.md's table has several errors, noted below). BACKGROUND is
/// the official `editor.background` (`#262335`) - THEME_ANALYSIS.md's guess
/// of `#261B2F` does not appear anywhere in the upstream file (that hex is
/// close to but not `sideBar.background`, `#241b2f`). FOREGROUND is the
/// official `foreground`/`terminal.foreground` (`#ffffff`) - THEME_ANALYSIS.md
/// got this one right. GREEN (`#72f1b8`, HTML/XML tag + `ansiBrightGreen`),
/// CYAN (`#36f9f6`, Function/Character escape), PINK (`#ff7edb`, Variable/
/// Support variable/`ansiBrightMagenta`), YELLOW (`#fede5d`, Storage/Keyword/
/// `ansiBrightYellow`), ORANGE (`#ff8b39`, String) are the official values
/// unchanged. RED is the official Entity/`errorForeground`/`ansiBrightRed`
/// colour (`#fe4450`), lightened for the contrast gate (see below) -
/// THEME_ANALYSIS.md mislabels this "keywords red"; the actual Keyword token
/// (`keyword`/`keyword.control`/`keyword.operator`) is `#fede5d`, the same
/// yellow used above, not `#fe4450`.
///
/// THEME_ANALYSIS.md's Comment guess (`#7F87C4`) does not match upstream
/// either: the actual Comment token colour is `#848bbd`. GREY is derived from
/// this comment hue (~233°), not the official value - see below.
/// THEME_ANALYSIS.md's "numbers pink `#F92AAD`" does not appear anywhere in
/// the upstream file at all; the Number/Constant token (`constant.numeric`/
/// `constant`) is `#f97e72`, a salmon-orange, not pink - not used here since
/// ORANGE already takes the (distinct) String colour `#ff8b39`.
///
/// PURPLE has no single canonical syntax token; it takes the official
/// `gitDecoration.modifiedResourceForeground`/`editorGutter.modifiedBackground`
/// accent (`#b893ce`), a lavender distinct in hue (~278°) from PINK's magenta
/// (~322°) and CYAN's cyan (~180°).
///
/// BLUE and BROWN have no SynthWave '84 accent (its neon set has no blue or
/// brown/tan hue - the theme's only "blue" tokens, `ansiBlue`/`terminal.
/// ansiBrightBlue` at `#03edf9`, are the same colour as CYAN); both are
/// derived:
/// - BLUE sits on the Comment token's own hue family (~233°, between CYAN's
///   ~180° and PURPLE's ~278°) at a bright, saturated neon lightness (s 55%,
///   l 68%) to match the theme's aesthetic -> `#818bda`.
/// - BROWN is the String/ORANGE hue (~25°) desaturated (s 45%, vs ORANGE's
///   100%) and darkened slightly (l 55%, vs ORANGE's 61%) to read as a tan
///   rather than another orange -> `#c08259`.
///
/// GREY has no usable official value: SynthWave '84's Comment token
/// (`#848bbd`, l 63%) sits at a hue too close to the derived BLUE to
/// distinguish once tuned (both live on the ~233° family) and, at its own
/// lightness, undershoots the softened-surface text floor. GREY is instead
/// derived on the same ~233° hue family, desaturated (s 18%, vs Comment's
/// 30%) and lightened (l 74%, vs Comment's 63%) -> `#b1b4c9`, clearing the
/// text/surface floors and staying >=15.0 deltaE from BLUE and PURPLE.
///
/// Tuning forced by `tests::gate_contrast_all_themes`: RED (official
/// `#fe4450`, l 63%) reaches only 4.48:1 via `contrast` (landing on
/// BACKGROUND, just under the 4.5:1 transform floor); lightened
/// (hue/saturation unchanged) to l 65% (`#fe4d59`), clearing it at 4.67:1.
/// GREY's derivation above already bakes in the gate-driven lightness (l 74%
/// was the minimum clearing 3:1 against `soften(FOREGROUND, 75)`, the
/// weakest surface, at l 72% it only reached 2.83:1). Every other official
/// value (GREEN/CYAN/PINK/YELLOW/ORANGE/PURPLE) clears every floor at its
/// native lightness.
pub static SYNTHWAVE_84: Palette = Palette {
    red: Color {
        r: 254,
        g: 77,
        b: 89,
    },
    green: Color {
        r: 114,
        g: 241,
        b: 184,
    },
    blue: Color {
        r: 129,
        g: 139,
        b: 218,
    },
    yellow: Color {
        r: 254,
        g: 222,
        b: 93,
    },
    purple: Color {
        r: 184,
        g: 147,
        b: 206,
    },
    cyan: Color {
        r: 54,
        g: 249,
        b: 246,
    },
    pink: Color {
        r: 255,
        g: 126,
        b: 219,
    },
    orange: Color {
        r: 255,
        g: 139,
        b: 57,
    },
    brown: Color {
        r: 192,
        g: 130,
        b: 89,
    },
    grey: Color {
        r: 177,
        g: 180,
        b: 201,
    },
    foreground: Color {
        r: 255,
        g: 255,
        b: 255,
    },
    background: Color {
        r: 38,
        g: 35,
        b: 53,
    },
};

/// PaperColor Light (Nikyle Nguyen / NLKNguyen, github.com/NLKNguyen/
/// papercolor-theme; verified 2026-07-13 directly against
/// `autoload/PaperColor.vim`'s `s:themes['default'].light.palette` -
/// THEME_ANALYSIS.md's light table matched upstream exactly, with no
/// discrepancies). BACKGROUND (`color00`, `#eeeeee`) and FOREGROUND
/// (`color07`, `#444444`) are the official values, unchanged. GREEN
/// (`color02`, `#008700`) and PURPLE (`color11`, `#8700af`) are official,
/// unchanged - both already clear every gate at native lightness.
///
/// RED takes `color01` (`#af0000`) rather than the brighter `color09`
/// (`#d70000`, "alternative error triggers") - `color01` is the primary
/// error/red token and already clears every floor; `color09` is not needed.
/// PINK takes the official magenta `color10` (`#d70087`). ORANGE takes the
/// official `color12`/`color13` (`#d75f00`, identical in both slots
/// upstream).
///
/// CYAN and BLUE both have to be derived from PaperColor's cluster of
/// blue-leaning teals (`color04` `#0087af`, `color06`/`color15` `#005f87`,
/// `color14` `#005faf`) - none of the three sit far enough apart in hue
/// (all land between ~193° and ~211°) to pass THEMING.md's BLUE/CYAN
/// distinctness gate as-is (`tests::gate_contrast_all_themes`'s 15.0
/// deltaE floor). CYAN takes `color04` (`#0087af`) unchanged - the most
/// cyan-leaning of the cluster. BLUE takes `color14` (`#005faf`, the most
/// blue-leaning) with saturation raised (76% -> 100%, hue/lightness
/// unchanged) to `#0037c7`, pushing it far enough from CYAN's hue to clear
/// 15.0 deltaE (see tuning below).
///
/// YELLOW has no PaperColor light token distinct from GREEN/ORANGE: the
/// palette's only yellow-ish colour is `color03` (`#5f8700`), a yellow-green
/// that sits too close in hue (~74°) to GREEN (~120°) to read as a separate
/// accent, and too close to ORANGE by deltaE once both are tuned for text.
/// YELLOW is instead derived as a dark gold (hue 45°, between ORANGE's ~26°
/// and `color03`'s ~74°), at a lightness suited to text on this light
/// BACKGROUND -> `#8a6d00` (see tuning below).
///
/// BROWN has no PaperColor accent; it is derived at ORANGE's hue (~26°)
/// desaturated and darkened to read as a tan/brown rather than another
/// orange (`#734723`, s 50%/l 30%, vs ORANGE's s 100%/l 42%), chosen to
/// clear 15.0 deltaE from ORANGE, YELLOW, and GREY.
///
/// GREY has no usable official value: `color05` (`#878787`, l 53%) manages
/// only 1.88:1 against `soften(FOREGROUND, 75)`, far under the 3:1 text
/// floor at this dark a checkerboard surface, and a plain darkened neutral
/// landed within 15.0 deltaE of the (also-dark) FOREGROUND at every
/// lightness that cleared the surface floor. GREY is instead moved to a
/// teal-grey hue (163°, between GREEN's 120° and CYAN's 194°, s 24%) at
/// l 31% -> `#395c4c`, clearing every softened surface at >=3:1 and 15.0
/// deltaE from both FOREGROUND and BROWN.
///
/// Tuning forced by `tests::gate_contrast_all_themes`:
/// - GREEN (official `color02` `#008700`, l 26%) reaches only 4.05:1 via
///   `contrast`, under the 4.5:1 transform floor; darkened (hue/saturation
///   unchanged) to l 20% (`#006600`), clearing it at 5.14:1.
/// - RED (official `color01` `#af0000`, l 34%) sat well clear of every
///   floor unchanged, but was re-checked after other slots moved and stayed
///   at its official value.
/// - CYAN (`color04` `#0087af`, l 34%) reached only 3.56:1 via `contrast`;
///   darkened (hue/saturation unchanged) to l 24% (`#005e7a`), clearing it
///   at 4.66:1.
/// - BLUE (`color14` `#005faf`, hue 211°/s 76%/l 34%) reached only 12.6
///   deltaE from CYAN's new, darker value - still under the 15.0 floor;
///   resaturating to 100% (hue/lightness unchanged) widens the perceptual
///   gap to `#0037c7`, clearing 18.3+ deltaE.
/// - PINK (official `color10` `#d70087`, l 42%) reached only 4.27:1 via
///   `contrast`; darkened (hue/saturation unchanged) to l 30% (`#990060`),
///   clearing it at 5.02:1.
/// - ORANGE (official `color12`/`color13` `#d75f00`, l 42%) reached only
///   3.28:1 via `contrast`; darkened (hue/saturation unchanged) to l 27%
///   (`#8a3d00`), clearing it at 4.86:1.
/// - YELLOW's derived starting point (hue 45°, s 100%, l 42% - ORANGE's
///   pre-tuning lightness) reached only 4.24:1 via `contrast`; darkened to
///   l 22% (`#705400`), clearing it at 5.86:1 while staying >=15.0 deltaE
///   from ORANGE and GREEN.
/// - GREY's derivation above already bakes in the gate-driven hue/lightness
///   choice: a neutral grey at any lightness clearing `soften(PINK, 75)`'s
///   3:1 floor landed within 15.0 deltaE of FOREGROUND; the teal hue and
///   l 31% clear both simultaneously.
pub static PAPERCOLOR_LIGHT: Palette = Palette {
    red: Color { r: 175, g: 0, b: 0 },
    green: Color { r: 0, g: 102, b: 0 },
    blue: Color {
        r: 0,
        g: 55,
        b: 199,
    },
    yellow: Color {
        r: 112,
        g: 84,
        b: 0,
    },
    purple: Color {
        r: 135,
        g: 0,
        b: 175,
    },
    cyan: Color {
        r: 0,
        g: 94,
        b: 122,
    },
    pink: Color {
        r: 153,
        g: 0,
        b: 96,
    },
    orange: Color {
        r: 138,
        g: 61,
        b: 0,
    },
    brown: Color {
        r: 115,
        g: 71,
        b: 35,
    },
    grey: Color {
        r: 57,
        g: 92,
        b: 76,
    },
    foreground: Color {
        r: 68,
        g: 68,
        b: 68,
    },
    background: Color {
        r: 238,
        g: 238,
        b: 238,
    },
};

/// PaperColor Dark (same source as PAPERCOLOR_LIGHT, verified 2026-07-13
/// against `s:themes['default'].dark.palette`). Note: THEME_ANALYSIS.md's
/// dark table has one transcription error - its `color16` row reads
/// `#5FADFT`, which is not valid hex; upstream's actual `color16` is
/// `#5fafd7` (byte-identical to `color04`, both labelled "Variable
/// structures"/"Object properties" - PaperColor reuses the same blue for
/// both slots). Every other value in THEME_ANALYSIS.md's dark table matched
/// upstream exactly.
///
/// BACKGROUND (`color00`, `#1c1c1c`) and FOREGROUND (`color07`, `#d0d0d0`)
/// are the official values, unchanged. GREEN takes `color02` (`#5faf00`),
/// unchanged - already clears every gate natively. PURPLE takes `color11`
/// (`#af87d7`), unchanged. CYAN takes `color14` (`#00afaf`), unchanged.
/// BLUE takes `color04`/`color16` (`#5fafd7`), unchanged.
///
/// RED cannot use `color01` (`#af005f`): that token is a pink-red at hue
/// 327°, only ~5° away from PINK's own hue (`color13` `#ff5faf`, hue 333°) -
/// nowhere near clearing THEMING.md's RED/PINK distinctness requirement
/// (`tests::gate_contrast_all_themes`'s 15.0 deltaE floor) at any lightness
/// on that hue. PaperColor dark has no true-red token at all (its whole warm
/// end runs pink-red -> orange-tan -> gold, skipping ~0-15°); RED is instead
/// derived at hue 350° (true red, rotated off `color01`'s pink-red toward
/// red), at `color01`'s own saturation (s 100%), tuned for the contrast gate
/// (see below).
///
/// ORANGE takes `color06` (`#d7875f`, "primary functions"), unchanged.
/// YELLOW cannot share `color06`'s territory or ORANGE's hue; it takes
/// `color17` (`#d7af00`, "specialized strings"), a true gold distinct from
/// ORANGE's tan - unchanged.
///
/// PINK takes `color13` (`#ff5faf`), unchanged.
///
/// BROWN has no PaperColor dark accent; it is derived at ORANGE's hue (~20°)
/// desaturated and lightened to read as a tan/brown rather than another
/// orange (`#b48268`, s 35%/l 56%, vs ORANGE's s 55%/l 61%), tuned for the
/// contrast gate (see below).
///
/// GREY is tuned, not the official `color05` (`#808080`, l 50%): it manages
/// only 2.05:1 against `soften(FOREGROUND, 75)`, under the 3:1 text floor at
/// this dark a BACKGROUND. GREY is lightened on the same neutral hue to
/// l 65% (`#a6a6a6`), clearing every softened surface at >=3:1 and 15.0
/// deltaE from FOREGROUND and BROWN.
///
/// Tuning forced by `tests::gate_contrast_all_themes`:
/// - RED's derived starting point (hue 350°, s 100%, l 44% - `#d4002c`)
///   reached only 4.46:1 via `contrast`, under the 4.5:1 transform floor;
///   an intermediate darkening to l 34% (`#ff0a3c` after gamut correction)
///   overshot in the wrong direction (3.55:1) - `contrast` was landing on
///   the opposite (BACKGROUND) side at that lightness, so darkening made it
///   worse, not better. Re-lightened to l 60% (`#ff325f`), clearing it at
///   4.9:1+ and staying >=15.0 deltaE from PINK.
/// - BROWN's derived starting point (`#8a5a42`, l 40%) reached only 2.94:1
///   against BACKGROUND, under the 3:1 text floor; lightened
///   (hue/saturation unchanged) to l 56% (`#b48268`), clearing it at 3.7:1+
///   while staying >=15.0 deltaE from ORANGE, YELLOW, and GREY.
pub static PAPERCOLOR_DARK: Palette = Palette {
    red: Color {
        r: 255,
        g: 50,
        b: 95,
    },
    green: Color {
        r: 95,
        g: 175,
        b: 0,
    },
    blue: Color {
        r: 95,
        g: 175,
        b: 215,
    },
    yellow: Color {
        r: 215,
        g: 175,
        b: 0,
    },
    purple: Color {
        r: 175,
        g: 135,
        b: 215,
    },
    cyan: Color {
        r: 0,
        g: 175,
        b: 175,
    },
    pink: Color {
        r: 255,
        g: 95,
        b: 175,
    },
    orange: Color {
        r: 215,
        g: 135,
        b: 95,
    },
    brown: Color {
        r: 180,
        g: 130,
        b: 104,
    },
    grey: Color {
        r: 166,
        g: 166,
        b: 166,
    },
    foreground: Color {
        r: 208,
        g: 208,
        b: 208,
    },
    background: Color {
        r: 28,
        g: 28,
        b: 28,
    },
};

/// Monokai (Wimer Hazenberg, originally for TextMate, later Sublime Text's
/// default; verified 2026-07-13 against the widely-mirrored
/// `monokai.tmTheme` values, which THEME_ANALYSIS.md's table matches exactly,
/// with no discrepancies). BACKGROUND (`#272822`), FOREGROUND (`#f8f8f2`),
/// GREEN (`#a6e22e`), ORANGE (`#fd971f`), YELLOW (`#e6db74`), PURPLE
/// (`#ae81ff`) are the official values, unchanged.
///
/// Monokai's hot-magenta accent (used for keywords/control flow) takes PINK,
/// and its light cyan-blue (`#66d9ef`, used for built-in types) takes CYAN -
/// CYAN is unchanged; PINK is lightened from the official `#f92672` (see
/// below). Monokai has no accent distinct from
/// either of those for RED or BLUE, and no brown accent at all; RED, BLUE,
/// and BROWN are all derived:
/// - RED sits at a true-red hue (5°) distinct from PINK's magenta (~338°) and
///   ORANGE's amber (~32°), at a similarly high saturation (s 90%) to match
///   Monokai's neon aesthetic; an initial l 58% (`#f44434`) only reached
///   4.05:1 via `contrast` (the 4.5:1 transform floor, landing on the
///   BACKGROUND side), so it was lightened to l 65% -> `#f66355`, clearing
///   it at 4.88:1.
/// - BLUE sits at a hue (225°) between CYAN (~190°) and PURPLE (~261°), at a
///   saturation (s 70%) chosen to read as a distinct mid-blue rather than a
///   blend; an initial l 65% (`#6786e4`) only reached 4.32:1 via `contrast`
///   (the 4.5:1 transform floor, landing on the BACKGROUND side), so it was
///   lightened to l 72% -> `#869fea`, clearing it at 5.82:1.
/// - BROWN is ORANGE's hue (32°) desaturated (s 40%, vs ORANGE's 98%) to
///   read as a tan/brown rather than another orange; an initial l 45%
///   (`#ab773b`) only reached 3.88:1 via `contrast` (the 4.5:1 transform
///   floor, landing on the BACKGROUND side), so it was lightened to l 55%
///   -> `#ba8f5e`, clearing it at 5.12:1.
///
/// GREY is tuned, not the official Comment colour: `#75715e` (l 41%) is too
/// dark to reach the 3:1 text floor against the softened checkerboard
/// surfaces on this dark BACKGROUND. Lightening on the same olive hue family
/// (50°, s 11%) reaches the 3:1 floor only around l 80% (`#d2d0c6`), but at
/// that lightness it lands within 15.0 deltaE of FOREGROUND (both being
/// pale, low-saturation, warm-hued colours - the same "hue shared with
/// FOREGROUND" bind seen in CATPPUCCIN_MOCHA/TOKYO_NIGHT). GREY is instead
/// moved to a cool near-neutral hue (200°, s 15%) at l 80% -> `#c4cfd4`,
/// clearing `soften(FOREGROUND, 75)` at 3.07:1 and staying 16.5 deltaE from
/// FOREGROUND (and far clear of BROWN, at 43+ deltaE).
///
/// PINK (official `#f92672`, l 56%) only reached 3.96:1 via `contrast` (the
/// 4.5:1 transform floor, landing on the BACKGROUND side); lightened
/// (hue/saturation unchanged) to l 64% -> `#fa4c8c`, clearing it at 4.62:1.
///
/// No further tuning was forced by `tests::gate_contrast_all_themes` beyond
/// GREY's, RED's, BLUE's, PINK's, and BROWN's tunings above.
pub static MONOKAI: Palette = Palette {
    red: Color {
        r: 246,
        g: 99,
        b: 85,
    },
    green: Color {
        r: 166,
        g: 226,
        b: 46,
    },
    blue: Color {
        r: 134,
        g: 159,
        b: 234,
    },
    yellow: Color {
        r: 230,
        g: 219,
        b: 116,
    },
    purple: Color {
        r: 174,
        g: 129,
        b: 255,
    },
    cyan: Color {
        r: 102,
        g: 217,
        b: 239,
    },
    pink: Color {
        r: 250,
        g: 76,
        b: 140,
    },
    orange: Color {
        r: 253,
        g: 151,
        b: 31,
    },
    brown: Color {
        r: 186,
        g: 143,
        b: 94,
    },
    grey: Color {
        r: 196,
        g: 207,
        b: 212,
    },
    foreground: Color {
        r: 248,
        g: 248,
        b: 242,
    },
    background: Color {
        r: 39,
        g: 40,
        b: 34,
    },
};

/// JetBrains Darcula (IntelliJ IDEA/PyCharm/WebStorm/Rider default dark
/// theme; not to be confused with the unrelated DRACULA theme above).
/// Verified 2026-07-13 against the widely-mirrored Darcula editor-scheme
/// values (bsobol/npp-darcula and other independent ports of the IntelliJ
/// scheme) - THEME_ANALYSIS.md's table is accurate for every value it lists
/// (bg `#2b2b2b`, fg `#a9b7c6`, comment `#808080`, keyword `#cc7832`,
/// string `#6a8759`, function gold `#ffc66d`, number `#6897bb`, annotation
/// mustard `#bbb529`); the mirrors also confirm constant purple `#9876aa`,
/// javadoc green `#629755`, and a Ruby-comment tan `#bc9458`.
///
/// BACKGROUND (`#2b2b2b`), BLUE (number blue `#6897bb`), YELLOW (function
/// gold `#ffc66d`), and BROWN (upstream's tan `#bc9458`) are official
/// values, unchanged. The annotation mustard (`#bbb529`) is unused: YELLOW
/// is already taken by the gold and mustard sits too close to it in hue to
/// serve any other slot.
///
/// CYAN and PINK have no Darcula accent; both are derived:
/// - CYAN is a soft steel cyan (hue 191°, s 37%, l 59% -> `#6fafbd`) in the
///   territory of Darcula's HTML-entity blue (`#6d9cbe`) but pulled to a
///   true cyan hue to stay >=15.0 deltaE from BLUE (16.4, vs. the 15.0
///   floor in `tests::gate_contrast_all_themes`).
/// - PINK is a dusty rose (hue 328°, s 49%, l 65% -> `#d179a8`) matching
///   Darcula's muted saturation range, distinct from RED (27.4 deltaE) and
///   PURPLE (22.6).
///
/// Tuning forced by `tests::gate_contrast_all_themes` (hue/saturation
/// unchanged in every case; each lightness is the minimum clearing the
/// forcing check):
/// - FOREGROUND (official `#a9b7c6`, l 72%) reaches only 4.18:1 via
///   `contrast(soften(FOREGROUND, 75))`, under the 4.5:1 transform floor
///   (the same FG-lightening precedent as ONE_DARK/GRUVBOX_DARK); lightened
///   to l 78% -> `#bcc7d2`, clearing it at 4.68:1.
/// - RED (Darcula's error red `#bc3f3c`, l 49%) reaches only 2.64:1 against
///   BACKGROUND, under both the 3:1 text floor and the 4.5:1 `contrast`
///   floor; lightened to l 65% -> `#d37876`, clearing both (4.53:1 via
///   `contrast`).
/// - GREEN (string green `#6a8759`, l 44%) reaches only 3.52:1 via
///   `contrast`; lightened to l 50% -> `#7a9b67`, clearing it at 4.53:1.
/// - PURPLE (constant purple `#9876aa`, l 57%) reaches only 3.71:1 via
///   `contrast`; lightened to l 62% -> `#a587b5`, clearing it at 4.54:1.
/// - ORANGE (keyword orange `#cc7832`, l 50%) reaches only 4.25:1 via
///   `contrast`; lightened to l 52% -> `#cf7f3c`, clearing it at 4.55:1.
/// - GREY (official comment `#808080`, l 50%) reaches only 2.16:1 against
///   `soften(FOREGROUND, 75)`, under the 3:1 text floor; lightened to l 63%
///   with a slight warm tint (hue 40°, s 5% - a pure-neutral lift landed
///   only 15.2 deltaE from the cool, lightened FOREGROUND, too tight
///   against the 15.0 floor) -> `#a5a29c`, clearing every softened surface
///   at 3.15:1+ and 16.7 deltaE from FOREGROUND.
pub static DARCULA: Palette = Palette {
    red: Color {
        r: 211,
        g: 120,
        b: 118,
    },
    green: Color {
        r: 122,
        g: 155,
        b: 103,
    },
    blue: Color {
        r: 104,
        g: 151,
        b: 187,
    },
    yellow: Color {
        r: 255,
        g: 198,
        b: 109,
    },
    purple: Color {
        r: 165,
        g: 135,
        b: 181,
    },
    cyan: Color {
        r: 111,
        g: 175,
        b: 189,
    },
    pink: Color {
        r: 209,
        g: 121,
        b: 168,
    },
    orange: Color {
        r: 207,
        g: 127,
        b: 60,
    },
    brown: Color {
        r: 188,
        g: 148,
        b: 88,
    },
    grey: Color {
        r: 165,
        g: 162,
        b: 156,
    },
    foreground: Color {
        r: 188,
        g: 199,
        b: 210,
    },
    background: Color {
        r: 43,
        g: 43,
        b: 43,
    },
};

/// VS Code Dark+ (Microsoft's classic built-in dark theme; verified
/// 2026-07-13 directly against `extensions/theme-defaults/themes/dark_plus.json`
/// and its base `dark_vs.json` in github.com/microsoft/vscode - not
/// THEME_ANALYSIS.md's table, which is largely accurate but misses one
/// per-variant distinction, see below). BACKGROUND (`editor.background`,
/// `#1e1e1e`) and FOREGROUND (`editor.foreground`, `#d4d4d4`) are the
/// official `dark_vs.json` values, unchanged. GREEN is the official Comment
/// colour (`#6a9955`); BLUE is the official keyword/entity-tag colour
/// (`#569cd6`); YELLOW is the official function-name colour (`#dcdcaa`);
/// CYAN is the official type/class colour (`#4ec9b0`, `support.type`);
/// ORANGE is the official string colour (`#ce9178`); PINK is the official
/// control-flow keyword colour (`#c586c0`, `keyword.control`) - THEME_ANALYSIS.md
/// calls this token "pink-magenta", which is why it lands on PINK rather than
/// PURPLE here. RED is the official `invalid` scope colour (`#f44747`) from
/// `dark_vs.json` - distinct from DARK_MODERN's RED, see below.
///
/// THEME_ANALYSIS.md's table omits the variable colour `#9cdcfe` entirely;
/// it isn't used here since PURPLE/BROWN/GREY already need to be derived and
/// `#9cdcfe`'s hue (~201°) sits too close to BLUE's (~207°) to serve as a
/// second slot.
///
/// PURPLE, BROWN, and GREY have no VS Code Dark+ token distinct enough from
/// the slots above and are derived:
/// - PURPLE sits at a violet hue (260°) between BLUE (207°) and PINK (305°),
///   at s 55%/l 68% -> `#9e81da`, clearing 15.0 deltaE from both RED and PINK
///   (21.8 from PINK, the tighter of the two) and the 4.5:1 `contrast`
///   transform floor.
/// - BROWN is a desaturated tan near ORANGE's hue (20°, vs ORANGE's own 17°)
///   at s 15%/l 60% -> `#a8948a`, clearing 15.0 deltaE from ORANGE and GREY
///   (a straight desaturation of ORANGE at s 25-30% collided with ORANGE
///   itself under 15.0 deltaE - VS Code's terracotta is already fairly
///   desaturated, so BROWN needs to move further than usual).
/// - GREY has no usable Dark+ token: the theme has no explicit
///   `editorLineNumber.foreground` (it inherits VS Code's undocumented
///   built-in default, not present in the theme file, so there is nothing
///   "official" to cite), and the nearby `entity.name.label` colour
///   (`#c8c8c8`) is a near-white too close to FOREGROUND. GREY is derived on
///   Dark Modern's own official line-number hue family (~215°, see
///   DARK_MODERN) at s 15%/l 62% -> `#909cad`, clearing every softened
///   surface at >=3:1 and 15.0 deltaE from BLUE (26.3, the closest hue).
///
/// No tuning beyond the derivations above was forced by
/// `tests::gate_contrast_all_themes`: every official value (RED/GREEN/BLUE/
/// YELLOW/CYAN/ORANGE/PINK) clears the 3:1 text floor and 4.5:1 `contrast`
/// transform floor (via BACKGROUND) at its native value.
pub static VS_CODE_DARK_PLUS: Palette = Palette {
    red: Color {
        r: 244,
        g: 71,
        b: 71,
    },
    green: Color {
        r: 106,
        g: 153,
        b: 85,
    },
    blue: Color {
        r: 86,
        g: 156,
        b: 214,
    },
    yellow: Color {
        r: 220,
        g: 220,
        b: 170,
    },
    purple: Color {
        r: 158,
        g: 129,
        b: 218,
    },
    cyan: Color {
        r: 78,
        g: 201,
        b: 176,
    },
    pink: Color {
        r: 197,
        g: 134,
        b: 192,
    },
    orange: Color {
        r: 206,
        g: 145,
        b: 120,
    },
    brown: Color {
        r: 168,
        g: 148,
        b: 138,
    },
    grey: Color {
        r: 144,
        g: 156,
        b: 173,
    },
    foreground: Color {
        r: 212,
        g: 212,
        b: 212,
    },
    background: Color {
        r: 30,
        g: 30,
        b: 30,
    },
};

/// VS Code Dark Modern (Microsoft's flatter, newer built-in dark theme;
/// verified 2026-07-13 against `extensions/theme-defaults/themes/dark_modern.json`,
/// which `"include"`s `dark_plus.json` for all `tokenColors` - meaning
/// Dark Modern's syntax palette is byte-identical to Dark+'s. GREEN, BLUE,
/// YELLOW, CYAN, ORANGE, PINK, and the derived PURPLE/BROWN/GREY are
/// therefore all identical to VS_CODE_DARK_PLUS - see that doc comment for
/// their sourcing. Only BACKGROUND, FOREGROUND, and RED differ between the
/// two variants, all confirmed directly in `dark_modern.json`'s `colors`
/// block:
/// - BACKGROUND (`editor.background`, `#1f1f1f`, vs Dark+'s `#1e1e1e`) and
///   FOREGROUND (`editor.foreground`, `#cccccc`, vs Dark+'s `#d4d4d4`) are
///   both explicitly overridden in `dark_modern.json`.
/// - RED is Dark Modern's own `errorForeground`/`editorGutter.deletedBackground`
///   (`#f85149`) rather than Dark+'s token-level `invalid` colour
///   (`#f44747`) - THEME_ANALYSIS.md's text calls out `#F85149` as a
///   possibility to check but its comparison table only lists one shared RED
///   for both variants, missing this distinction.
///
/// Every accent was independently re-checked against this lighter
/// BACKGROUND per the usual "a hue passing on one background can fail on
/// another" rule (see e.g. TOKYO_NIGHT_STORM); all clear unchanged. No
/// tuning was forced by `tests::gate_contrast_all_themes` beyond the
/// PURPLE/BROWN/GREY derivations already documented on VS_CODE_DARK_PLUS.
pub static VS_CODE_DARK_MODERN: Palette = Palette {
    red: Color {
        r: 248,
        g: 81,
        b: 73,
    },
    foreground: Color {
        r: 204,
        g: 204,
        b: 204,
    },
    background: Color {
        r: 31,
        g: 31,
        b: 31,
    },
    ..VS_CODE_DARK_PLUS
};

/// brdgme Light Deuteranopia/Protanopia (see `LIGHT_PROTANOPIA`, which shares
/// this exact palette - both CVD types are red/green cone deficiencies with
/// near-identical practical confusion lines, and this palette clears both
/// simulated-vision gates at once, so a single tuned set covers both per the
/// task brief). Hues are seeded from Okabe & Ito's colour-blind-safe palette
/// (jfly.uni-koeln.de/color; orange `#E69F00`, sky blue `#56B4E9`, bluish
/// green `#009E73`, yellow `#F0E442`, blue `#0072B2`, vermillion `#D55E00`,
/// reddish purple `#CC79A7`) mapped RED<-vermillion, ORANGE<-orange,
/// BLUE<-blue, CYAN<-sky blue, GREEN<-bluish green, YELLOW<-yellow, and
/// PURPLE/PINK split across the reddish-purple family (violet vs. magenta);
/// BROWN is a desaturated warm tone near RED/ORANGE's hue, GREY a
/// near-neutral. Values are not the Okabe-Ito hexes verbatim: this slot set
/// needs 9 hues (Okabe-Ito has 7 usable accents) plus BROWN/GREY, all forced
/// to satisfy `tests::gate_contrast_all_themes` (3:1 text / 4.5:1 transform /
/// 15.0 deltaE distinctness in normal vision) *and* `tests::gate_cvd_simulation`
/// (>=9.0 deltaE pairwise on the 8 player colours + GREY + FOREGROUND, after
/// simulating deuteranopia and protanopia) simultaneously - so every hue was
/// darkened off Okabe-Ito's native (light-mode-unfriendly) lightness and
/// lightness-spaced within its own red/orange/yellow and purple/pink
/// sub-families (rather than relying on hue alone) via numerical search, since
/// deuteranopia/protanopia collapse much of the hue-only separation between
/// nearby warm hues and between RED/BROWN. PINK in particular needed a
/// distinctly different hue (~300°, violet-magenta) from a naive Okabe-Ito
/// magenta: at the initial candidate hue/lightness PINK and GREY simulated to
/// within 0.77 deltaE of each other under deuteranopia (`tests::
/// gate_cvd_simulation`), forcing the hue move. Achieved minima: deuteranopia
/// 15.33 deltaE (blue/purple), protanopia 15.43 deltaE (red/brown) - both
/// comfortably above the 9.0 floor `tests::gate_cvd_simulation` was
/// calibrated from (the worst of all 6 new CVD themes' achieved minima is
/// 11.06, on LIGHT_TRITANOPIA; see `tests::CVD_DISTINCT_DELTA_E`'s doc
/// comment for the full table). This palette's own minima are unchanged by
/// the `tests::simulate_cvd` coefficient fix (see that function's doc
/// comment) - the numbers above were re-measured under the corrected
/// simulation and still clear with margin; no colours here needed
/// re-tuning.
pub static LIGHT_DEUTERANOPIA: Palette = Palette {
    red: Color {
        r: 189,
        g: 28,
        b: 7,
    },
    green: Color {
        r: 22,
        g: 55,
        b: 46,
    },
    blue: Color {
        r: 14,
        g: 63,
        b: 142,
    },
    yellow: Color {
        r: 157,
        g: 171,
        b: 8,
    },
    purple: Color {
        r: 117,
        g: 69,
        b: 190,
    },
    cyan: Color {
        r: 7,
        g: 95,
        b: 146,
    },
    pink: Color {
        r: 109,
        g: 39,
        b: 109,
    },
    orange: Color {
        r: 199,
        g: 152,
        b: 52,
    },
    brown: Color {
        r: 124,
        g: 39,
        b: 24,
    },
    grey: Color {
        r: 122,
        g: 90,
        b: 80,
    },
    foreground: Color { r: 0, g: 0, b: 0 },
    background: Color {
        r: 255,
        g: 255,
        b: 255,
    },
};

/// brdgme Light Protanopia. Byte-identical to `LIGHT_DEUTERANOPIA` - see that
/// static's doc comment for full derivation and CVD calibration notes; both
/// CVD types were optimised together against the same palette and both clear
/// `tests::gate_cvd_simulation` independently (deuteranopia 15.33 deltaE,
/// protanopia 15.43 deltaE).
pub static LIGHT_PROTANOPIA: Palette = Palette {
    red: LIGHT_DEUTERANOPIA.red,
    green: LIGHT_DEUTERANOPIA.green,
    blue: LIGHT_DEUTERANOPIA.blue,
    yellow: LIGHT_DEUTERANOPIA.yellow,
    purple: LIGHT_DEUTERANOPIA.purple,
    cyan: LIGHT_DEUTERANOPIA.cyan,
    pink: LIGHT_DEUTERANOPIA.pink,
    orange: LIGHT_DEUTERANOPIA.orange,
    brown: LIGHT_DEUTERANOPIA.brown,
    grey: LIGHT_DEUTERANOPIA.grey,
    foreground: LIGHT_DEUTERANOPIA.foreground,
    background: LIGHT_DEUTERANOPIA.background,
};

/// brdgme Dark Deuteranopia/Protanopia (see `DARK_PROTANOPIA`, sharing this
/// palette for the same reason as `LIGHT_DEUTERANOPIA`/`LIGHT_PROTANOPIA`).
/// Same Okabe-Ito-derived hue mapping and slot derivation as
/// `LIGHT_DEUTERANOPIA` (RED<-vermillion, ORANGE<-orange, BLUE<-blue,
/// CYAN<-sky blue, GREEN<-bluish green, YELLOW<-yellow, PURPLE/PINK split
/// across the reddish-purple family, BROWN/GREY derived), independently
/// re-tuned (lightened, not just inverted) for the dark BACKGROUND
/// (`#121212`, brdgme's standard dark canvas) and FOREGROUND (`#ffffff`) -
/// a hue set passing on light can fail on dark and vice versa, the same
/// principle as every other light/dark pair in this file. Every hue needed
/// independent lightness tuning to clear `tests::gate_contrast_all_themes`
/// and `tests::gate_cvd_simulation` (>=9.0 deltaE post-simulation) at once;
/// PURPLE, CYAN and GREY needed re-tuning beyond the original search when a
/// matched-matrix fix to `tests::simulate_cvd`'s coefficients (see that
/// function's doc comment) changed the simulated result: the original
/// PURPLE/CYAN landed only 7.37 deltaE apart under corrected deuteranopia
/// simulation. Achieved minima under the corrected simulation: deuteranopia
/// 12.75 deltaE (green/grey), protanopia 13.06 deltaE (cyan/pink), both
/// comfortably above the 9.0 floor.
pub static DARK_DEUTERANOPIA: Palette = Palette {
    red: Color {
        r: 235,
        g: 85,
        b: 71,
    },
    green: Color {
        r: 66,
        g: 240,
        b: 195,
    },
    blue: Color {
        r: 104,
        g: 123,
        b: 252,
    },
    yellow: Color {
        r: 246,
        g: 237,
        b: 141,
    },
    purple: Color {
        r: 162,
        g: 164,
        b: 240,
    },
    cyan: Color {
        r: 63,
        g: 220,
        b: 252,
    },
    pink: Color {
        r: 222,
        g: 168,
        b: 226,
    },
    orange: Color {
        r: 250,
        g: 207,
        b: 129,
    },
    brown: Color {
        r: 185,
        g: 114,
        b: 104,
    },
    grey: Color {
        r: 179,
        g: 170,
        b: 165,
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

/// brdgme Dark Protanopia. Byte-identical to `DARK_DEUTERANOPIA` - see that
/// static's doc comment for full derivation and CVD calibration notes.
pub static DARK_PROTANOPIA: Palette = Palette {
    red: DARK_DEUTERANOPIA.red,
    green: DARK_DEUTERANOPIA.green,
    blue: DARK_DEUTERANOPIA.blue,
    yellow: DARK_DEUTERANOPIA.yellow,
    purple: DARK_DEUTERANOPIA.purple,
    cyan: DARK_DEUTERANOPIA.cyan,
    pink: DARK_DEUTERANOPIA.pink,
    orange: DARK_DEUTERANOPIA.orange,
    brown: DARK_DEUTERANOPIA.brown,
    grey: DARK_DEUTERANOPIA.grey,
    foreground: DARK_DEUTERANOPIA.foreground,
    background: DARK_DEUTERANOPIA.background,
};

/// brdgme Light Tritanopia. Same Okabe-Ito-derived hue mapping as
/// `LIGHT_DEUTERANOPIA` (RED<-vermillion, ORANGE<-orange, BLUE<-blue,
/// CYAN<-sky blue, GREEN<-bluish green, YELLOW<-yellow, PURPLE/PINK split,
/// BROWN/GREY derived), independently re-tuned: tritanopia confuses the
/// blue/yellow axis rather than red/green, so this palette leans on
/// lightness and red/green-axis separation instead - BLUE and YELLOW in
/// particular are pushed to opposite lightness extremes (BLUE very dark,
/// `l~20%`; YELLOW a muted mid-tone) rather than relying on their hue
/// difference, which tritanopia substantially erodes. Tuned by numerical
/// search against `tests::gate_contrast_all_themes` (normal-vision floors)
/// and `tests::gate_cvd_simulation` (>=9.0 deltaE post-tritanopia-simulation)
/// simultaneously. RED, ORANGE, BROWN and GREY needed re-tuning beyond the
/// original search when a matched-matrix fix to `tests::simulate_cvd`'s
/// coefficients (see that function's doc comment) changed the simulated
/// result: the original RED/BROWN landed only 3.56 deltaE apart under
/// corrected tritanopia simulation. Achieved minimum under the corrected
/// simulation: tritanopia 11.06 deltaE (brown/grey), comfortably above the
/// 9.0 floor.
pub static LIGHT_TRITANOPIA: Palette = Palette {
    red: Color {
        r: 197,
        g: 17,
        b: 0,
    },
    green: Color {
        r: 26,
        g: 75,
        b: 60,
    },
    blue: Color {
        r: 7,
        g: 20,
        b: 108,
    },
    yellow: Color {
        r: 167,
        g: 162,
        b: 14,
    },
    purple: Color {
        r: 95,
        g: 53,
        b: 139,
    },
    cyan: Color {
        r: 13,
        g: 135,
        b: 206,
    },
    pink: Color { r: 83, g: 5, b: 80 },
    orange: Color {
        r: 215,
        g: 175,
        b: 42,
    },
    brown: Color {
        r: 146,
        g: 46,
        b: 26,
    },
    grey: Color {
        r: 95,
        g: 82,
        b: 72,
    },
    foreground: Color { r: 0, g: 0, b: 0 },
    background: Color {
        r: 255,
        g: 255,
        b: 255,
    },
};

/// brdgme Dark Tritanopia. Same hue mapping and blue/yellow-avoidance
/// strategy as `LIGHT_TRITANOPIA` (see that static's doc comment),
/// independently re-tuned for the dark BACKGROUND (`#121212`)/FOREGROUND
/// (`#ffffff`) pair - lightened rather than darkened, with BLUE and YELLOW
/// again pushed toward opposite lightness extremes relative to each other.
/// Tuned against the same two gates as `LIGHT_TRITANOPIA`. PURPLE, PINK,
/// ORANGE and GREY needed re-tuning beyond the original search when a
/// matched-matrix fix to `tests::simulate_cvd`'s coefficients (see that
/// function's doc comment) changed the simulated result: the original
/// BROWN/GREY landed only 0.95 deltaE apart (effectively indistinguishable)
/// under corrected tritanopia simulation, with ORANGE/PINK and PURPLE/GREY
/// also failing. BROWN itself is unchanged from the original search.
/// Achieved minimum under the corrected simulation: tritanopia 11.58 deltaE
/// (purple/brown), comfortably above the 9.0 floor.
pub static DARK_TRITANOPIA: Palette = Palette {
    red: Color {
        r: 242,
        g: 143,
        b: 107,
    },
    green: Color {
        r: 119,
        g: 251,
        b: 183,
    },
    blue: Color {
        r: 26,
        g: 94,
        b: 253,
    },
    yellow: Color {
        r: 239,
        g: 232,
        b: 75,
    },
    purple: Color {
        r: 192,
        g: 84,
        b: 220,
    },
    cyan: Color {
        r: 41,
        g: 159,
        b: 215,
    },
    pink: Color {
        r: 246,
        g: 220,
        b: 237,
    },
    orange: Color {
        r: 248,
        g: 206,
        b: 157,
    },
    brown: Color {
        r: 191,
        g: 154,
        b: 120,
    },
    grey: Color {
        r: 193,
        g: 180,
        b: 180,
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

/// Modus Operandi Tritanopia (protesilaos/modus-themes, `modus-themes.el`,
/// `defconst modus-themes-operandi-tritanopia-palette`; verified against a
/// fresh fetch of `main`, 2026-07-14). 11 of 12 slots are Modus's own named
/// colours, unchanged: RED `red-faint` `#702000`, GREEN `green` `#006800`,
/// BLUE `blue` `#0031a9`, YELLOW `yellow` `#695500`, PURPLE `magenta-cooler`
/// `#531ab6`, CYAN `cyan` `#005e8b`, PINK `magenta-warmer` `#8f0075`, ORANGE
/// `red-warmer` `#b21100`, GREY `fg-dim` `#595959`, FOREGROUND `fg-main`
/// `#000000`, BACKGROUND `bg-main` `#ffffff` - all of Modus's own >=7:1 AAA
/// text-contrast claim against `bg-main`, preserved intact.
///
/// BROWN has no non-colliding stock Modus swap: the nearest candidate,
/// `yellow-faint` (`#624416`), simulates to within 0.54 deltaE of RED
/// (`red-faint`) under tritanopia - both collapse to a near-identical warm
/// grey once the S-cone signal is gone, differing only in the R/G channel
/// magnitude tritanopia keeps, which the two source lightnesses happen to
/// share. Modus's `olive` (`#4c6000`) very nearly fixes this (9.88 deltaE,
/// still short of the 10.0 floor) but a real fix needs the deficiency
/// tritanopia doesn't erase - post-simulation *lightness* separation, not
/// hue - so BROWN here is derived off RED's own hue (`red-faint`'s ~17°)
/// desaturated from 100% to 30% at the same 22% lightness -> `#493227`
/// (hue 20°/sat 30%/light 22%, numerically searched): heavily desaturating a
/// hue that's otherwise identical to RED changes its post-tritanopia
/// simulated *lightness* enough (63 vs. RED's simulated 83, both R=G/B=0
/// "yellow" after the S-cone loss) to separate the two by 11.40 deltaE,
/// comfortably reading as a warm coffee-brown at 11.87:1 contrast against
/// `bg-main` (still clears Modus's own 7:1 AAA bar).
///
/// Achieved minima: normal-vision 20.41 deltaE (brown/grey, `tests::
/// gate_contrast_all_themes`); tritanopia-simulated 10.66 deltaE (`tests::
/// gate_cvd_simulation`) - not triggered by BROWN (its nearest simulated
/// neighbour is RED at 11.40) but by RED/PINK (`red-faint`/`magenta-warmer`),
/// an inherent property of those two stock Modus values that was already
/// present before any BROWN derivation and is out of this theme's scope to
/// change. 10.66 clears the 10.0 floor but is the tightest CVD margin of any
/// theme in this file to date (previous tightest: LIGHT_TRITANOPIA's 11.06) -
/// flagged here in case a future `simulate_cvd` coefficient correction (as
/// already happened once, see that function's doc comment) needs to re-check
/// it.
pub static MODUS_OPERANDI_TRITANOPIA: Palette = Palette {
    red: Color {
        r: 112,
        g: 32,
        b: 0,
    },
    green: Color { r: 0, g: 104, b: 0 },
    blue: Color {
        r: 0,
        g: 49,
        b: 169,
    },
    yellow: Color {
        r: 105,
        g: 85,
        b: 0,
    },
    purple: Color {
        r: 83,
        g: 26,
        b: 182,
    },
    cyan: Color {
        r: 0,
        g: 94,
        b: 139,
    },
    pink: Color {
        r: 143,
        g: 0,
        b: 117,
    },
    orange: Color {
        r: 178,
        g: 17,
        b: 0,
    },
    brown: Color {
        r: 73,
        g: 50,
        b: 39,
    },
    grey: Color {
        r: 89,
        g: 89,
        b: 89,
    },
    foreground: Color { r: 0, g: 0, b: 0 },
    background: Color {
        r: 255,
        g: 255,
        b: 255,
    },
};

/// Modus Vivendi Tritanopia (protesilaos/modus-themes, `modus-themes.el`,
/// `defconst modus-themes-vivendi-tritanopia-palette`; verified against the
/// same fresh fetch as `MODUS_OPERANDI_TRITANOPIA`). 9 of 12 slots are Modus's
/// own named colours, unchanged: RED `red` `#ff5f59`, GREEN `green` `#44bc44`,
/// BLUE `indigo` `#9099d9`, YELLOW `yellow` `#cabf00`, PURPLE `magenta-cooler`
/// `#b6a0ff`, CYAN `cyan-cooler` `#6ae4b9`, PINK `maroon` `#cf7fa7`, GREY
/// `fg-dim` `#989898`, FOREGROUND `fg-main` `#ffffff`, BACKGROUND `bg-main`
/// `#000000`.
///
/// The stock swap fails the tritanopia gate on two pairs at once (this
/// palette has less slack than its light sibling above - `indigo` is already
/// spent on BLUE, leaving no untouched blue-ish stock hue to lean on):
/// - ORANGE (`yellow-warmer` `#ffa00f`) sits only 8.31 deltaE from
///   RED (`red`) post-simulation - short of the 10.0 floor by 1.69.
/// - BROWN (`gold` `#c0965b`) collapses to 0.48 deltaE from PINK (`maroon`) -
///   both are warm mid-lightness hues that tritanopia flattens to nearly the
///   same simulated colour.
///
/// ORANGE is a minimal nudge, not an invented hue: same hue (36°) and
/// saturation (100%) as stock `yellow-warmer`, lightened from l 53% to l 65%
/// (`#ffa00f` -> `#ffb84d`). `yellow-warmer`'s own lightness put its
/// post-simulation brightness too close to RED's; lightening it further
/// separates the two (12.41 deltaE from RED post-simulation) while staying
/// on Modus's own orange hue, at 12.22:1 contrast against `bg-main`.
///
/// BROWN has no viable stock swap: every Modus named warm hue at
/// `gold`/`rust`'s lightness collides with either PINK or (once ORANGE moved)
/// something else post-simulation, and an unconstrained numerical search's
/// only clean pass landed at a pale yellow-green (hue 60°, `#f2f2a1`) that
/// reads as a second yellow, not a brown - rejected as not in-character (see
/// WP3 research notes). Derived instead, same "desaturate near RED/ORANGE's
/// hue family" strategy as `MODUS_OPERANDI_TRITANOPIA`'s BROWN: hue 31°
/// (between stock `gold`'s 35° and `rust`'s 13°), sat 27%, light 46% ->
/// `#957656`, a muted tan/coffee brown clearing PINK by 19.00 deltaE
/// post-simulation at 5.00:1 contrast against `bg-main`.
///
/// Achieved minima: normal-vision 20.48 deltaE (blue/purple, `tests::
/// gate_contrast_all_themes`); tritanopia-simulated 10.83 deltaE (`tests::
/// gate_cvd_simulation`) - like `MODUS_OPERANDI_TRITANOPIA`, not triggered by
/// either derived slot (ORANGE's nearest simulated neighbour is RED at 12.41;
/// BROWN's is PINK at 19.00) but by RED/PINK (`red`/`maroon`), the same
/// inherent stock-Modus constraint noted on that palette, independently
/// present here under a different pair of hex values. 10.83 clears the 10.0
/// floor but, like its light sibling, is one of the tightest CVD margins in
/// this file - flagged for the same reason.
pub static MODUS_VIVENDI_TRITANOPIA: Palette = Palette {
    red: Color {
        r: 255,
        g: 95,
        b: 89,
    },
    green: Color {
        r: 68,
        g: 188,
        b: 68,
    },
    blue: Color {
        r: 144,
        g: 153,
        b: 217,
    },
    yellow: Color {
        r: 202,
        g: 191,
        b: 0,
    },
    purple: Color {
        r: 182,
        g: 160,
        b: 255,
    },
    cyan: Color {
        r: 106,
        g: 228,
        b: 185,
    },
    pink: Color {
        r: 207,
        g: 127,
        b: 167,
    },
    orange: Color {
        r: 255,
        g: 184,
        b: 77,
    },
    brown: Color {
        r: 149,
        g: 118,
        b: 86,
    },
    grey: Color {
        r: 152,
        g: 152,
        b: 152,
    },
    foreground: Color {
        r: 255,
        g: 255,
        b: 255,
    },
    background: Color { r: 0, g: 0, b: 0 },
};

/// Grouping used by the web theme picker to sort/section the registry. Every
/// theme has exactly one category - none of the five overlap in practice for
/// this theme set (see `DeutanProtan`'s doc comment).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeCategory {
    /// The two brdgme base themes (light/dark). Renders first in the picker
    /// with no heading.
    Default,
    /// Non-default, non-CVD themes with a light background. Displayed as
    /// "Light".
    Light,
    /// Non-default, non-CVD themes with a dark background. Displayed as
    /// "Dark".
    Dark,
    /// Deuteranopia- and protanopia-targeted themes (near-identical
    /// palettes in practice), grouped together. Displayed as "Deuteranopia
    /// / Protanopia".
    DeutanProtan,
    /// Tritanopia-targeted themes. Displayed as "Tritanopia".
    Tritan,
}

/// The set of registered themes, in display order. Light/Dark is assigned by
/// each palette's actual `background` lightness (see `rgb_to_hsl`), not by
/// theme name.
pub fn themes() -> &'static [(&'static str, ThemeCategory, &'static Palette)] {
    use ThemeCategory::{Dark, Default as DefaultCat, DeutanProtan, Light, Tritan};
    static THEMES: [(&str, ThemeCategory, &Palette); 34] = [
        ("brdgme light", DefaultCat, &LIGHT),
        ("brdgme dark", DefaultCat, &DARK),
        ("dracula", Dark, &DRACULA),
        ("alucard", Light, &ALUCARD),
        ("solarized dark", Dark, &SOLARIZED_DARK),
        ("solarized light", Light, &SOLARIZED_LIGHT),
        ("nord dark", Dark, &NORD_DARK),
        ("nord light", Light, &NORD_LIGHT),
        ("one dark", Dark, &ONE_DARK),
        ("one light", Light, &ONE_LIGHT),
        ("gruvbox dark", Dark, &GRUVBOX_DARK),
        ("gruvbox light", Light, &GRUVBOX_LIGHT),
        ("catppuccin mocha", Dark, &CATPPUCCIN_MOCHA),
        ("catppuccin latte", Light, &CATPPUCCIN_LATTE),
        ("tokyo night", Dark, &TOKYO_NIGHT),
        ("tokyo night storm", Dark, &TOKYO_NIGHT_STORM),
        ("tokyo night light", Light, &TOKYO_NIGHT_LIGHT),
        ("night owl", Dark, &NIGHT_OWL),
        ("light owl", Light, &LIGHT_OWL),
        ("synthwave 84", Dark, &SYNTHWAVE_84),
        ("papercolor light", Light, &PAPERCOLOR_LIGHT),
        ("papercolor dark", Dark, &PAPERCOLOR_DARK),
        ("monokai", Dark, &MONOKAI),
        ("darcula", Dark, &DARCULA),
        ("vs code dark plus", Dark, &VS_CODE_DARK_PLUS),
        ("vs code dark modern", Dark, &VS_CODE_DARK_MODERN),
        (
            "brdgme light deuteranopia",
            DeutanProtan,
            &LIGHT_DEUTERANOPIA,
        ),
        ("brdgme light protanopia", DeutanProtan, &LIGHT_PROTANOPIA),
        ("brdgme light tritanopia", Tritan, &LIGHT_TRITANOPIA),
        ("brdgme dark deuteranopia", DeutanProtan, &DARK_DEUTERANOPIA),
        ("brdgme dark protanopia", DeutanProtan, &DARK_PROTANOPIA),
        ("brdgme dark tritanopia", Tritan, &DARK_TRITANOPIA),
        (
            "modus operandi tritanopia",
            Tritan,
            &MODUS_OPERANDI_TRITANOPIA,
        ),
        (
            "modus vivendi tritanopia",
            Tritan,
            &MODUS_VIVENDI_TRITANOPIA,
        ),
    ];
    &THEMES
}

/// Rounds a 0..=255 scale value half-up.
fn round_u8(v: f64) -> u8 {
    (v + 0.5).floor().clamp(0.0, 255.0) as u8
}

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
            soften(LIGHT.foreground, 90, LIGHT.background).hex(),
            "#e6e6e6"
        );
        assert_eq!(
            soften(LIGHT.foreground, 80, LIGHT.background).hex(),
            "#cccccc"
        );
        assert_eq!(soften(LIGHT.pink, 80, LIGHT.background).hex(), "#f3d1de");
    }

    #[test]
    fn mix_exactness() {
        assert_eq!(
            mix(DRACULA.foreground, DRACULA.background, 0),
            DRACULA.foreground
        );
        assert_eq!(
            mix(DRACULA.foreground, DRACULA.background, 100),
            DRACULA.background
        );
        assert_eq!(
            mix(DRACULA.foreground, DRACULA.background, 86).hex(),
            "#454751"
        );
        assert_eq!(
            mix(DRACULA.foreground, DRACULA.background, 90).hex(),
            "#3d3f49"
        );
    }

    #[test]
    fn soften_is_a_mix_to_background() {
        assert_eq!(
            soften(DRACULA.foreground, 86, DRACULA.background),
            mix(DRACULA.foreground, DRACULA.background, 86)
        );
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

    // Names for `Palette::player_colors()`'s 8 slots, in that method's order.
    const PLAYER_NAMES: [&str; 8] = [
        "green", "red", "blue", "orange", "purple", "brown", "cyan", "pink",
    ];

    const TEXT_FLOOR: f64 = 3.0;
    const TRANSFORM_FLOOR: f64 = 4.5;
    // LIGHT's minimum pairwise player deltaE (brown vs grey) measures ~19.02
    // (see below); LIGHT is definitionally valid, so the threshold is set at
    // a round number safely below that.
    const DISTINCT_DELTA_E: f64 = 15.0;

    fn in_use_surfaces(palette: &Palette) -> Vec<(String, Color)> {
        let softens = crate::css::IN_USE_SOFTENS.iter().map(|&(named, pct)| {
            (
                format!("soften({}, {})", named, pct),
                soften(palette.color(named), pct, palette.background),
            )
        });
        let mixes = crate::css::IN_USE_MIXES
            .iter()
            .map(|&(source, target, pct)| {
                (
                    format!("mix({}, {}, {})", source, target, pct),
                    mix(palette.color(source), palette.color(target), pct),
                )
            });
        softens.chain(mixes).collect()
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
        for (theme_name, _, palette) in crate::themes() {
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

    // --- CVD (colour vision deficiency) simulation gate ---
    //
    // Dichromat simulation per Viénot, Brettel & Mollon 1999 ("Digital video
    // colourmaps for checking the legibility of displays by dichromats"):
    // linearise sRGB, transform to LMS cone space via the unnormalized
    // Hunt-Pointer-Estevez matrix used in that paper, then project out the
    // missing cone response using a fixed anchor-based substitution. The
    // substitution constants below are the classic Vischeck/daltonize-family
    // values that are matched to this exact *unnormalized* HPE matrix (as
    // opposed to the differently-scaled constants some D65-normalized-matrix
    // implementations use, which are numerically incompatible with the
    // matrix below and were the source of a prior bug here: pairing them
    // with this matrix made simulated white render as pure cyan instead of
    // staying achromatic - see `cvd_simulation_preserves_achromatic`).
    // Tritanopia has no simple Viénot 1999 formula (that paper only covers
    // the far more common red-green deficiencies); the tritanopia
    // coefficients below are the equivalent Brettel-et-al.-derived S-cone
    // projection used by the same family of simulators, applied in the same
    // LMS space - "Viénot-style", per the task brief.

    /// Linear-sRGB (D65) -> LMS, Hunt-Pointer-Estevez matrix as used by
    /// Viénot et al. 1999's simulation.
    const RGB_TO_LMS: [[f64; 3]; 3] = [
        [17.8824, 43.5161, 4.11935],
        [3.45565, 27.1554, 3.86714],
        [0.0299566, 0.184309, 1.46709],
    ];

    /// Inverse of `RGB_TO_LMS`: LMS -> linear-sRGB (D65).
    const LMS_TO_RGB: [[f64; 3]; 3] = [
        [0.0809444479, -0.130504409, 0.116721066],
        [-0.0102485335, 0.0540193266, -0.113614708],
        [-0.000365296938, -0.00412161469, 0.693511405],
    ];

    fn matvec(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
        [
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ]
    }

    fn srgb_to_linear(v: u8) -> f64 {
        let c = v as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn linear_to_srgb_u8(c: f64) -> u8 {
        let c = c.clamp(0.0, 1.0);
        let v = if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Cvd {
        Protanopia,
        Deuteranopia,
        Tritanopia,
    }

    /// Simulates how `c` would appear to a dichromat, per the Viénot-style
    /// LMS projection documented above.
    fn simulate_cvd(c: Color, kind: Cvd) -> Color {
        let lin = [
            srgb_to_linear(c.r),
            srgb_to_linear(c.g),
            srgb_to_linear(c.b),
        ];
        let lms = matvec(RGB_TO_LMS, lin);
        let (l, m, s) = (lms[0], lms[1], lms[2]);
        let lms_sim = match kind {
            // Missing L cone: reconstruct L from M and S.
            Cvd::Protanopia => [2.02344 * m - 2.52581 * s, m, s],
            // Missing M cone: reconstruct M from L and S.
            Cvd::Deuteranopia => [l, 0.494207 * l + 1.24827 * s, s],
            // Missing S cone: reconstruct S from L and M.
            Cvd::Tritanopia => [l, m, -0.395913 * l + 0.801109 * m],
        };
        let lin_sim = matvec(LMS_TO_RGB, lms_sim);
        Color {
            r: linear_to_srgb_u8(lin_sim[0]),
            g: linear_to_srgb_u8(lin_sim[1]),
            b: linear_to_srgb_u8(lin_sim[2]),
        }
    }

    /// Pins the regression the mismatched-matrix bug caused: achromatic
    /// (grey-scale) colours must stay achromatic under simulation, since a
    /// dichromat's remaining two cones still respond equally to a colour
    /// with no chroma. Simulating white with the old, mismatched
    /// normalized-matrix coefficients produced pure cyan (0, 255, 255).
    #[test]
    fn cvd_simulation_preserves_achromatic() {
        for kind in [Cvd::Protanopia, Cvd::Deuteranopia, Cvd::Tritanopia] {
            for c in [
                Color {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                Color {
                    r: 128,
                    g: 128,
                    b: 128,
                },
                Color { r: 0, g: 0, b: 0 },
            ] {
                let sim = simulate_cvd(c, kind);
                let max = sim.r.max(sim.g).max(sim.b);
                let min = sim.r.min(sim.g).min(sim.b);
                assert!(
                    max - min <= 8,
                    "{:?}-simulated {:?} -> {:?} is not near-achromatic (spread {})",
                    kind,
                    c,
                    sim,
                    max - min
                );
            }
        }
    }

    /// Floor for pairwise deltaE among the 8 player colours + GREY +
    /// FOREGROUND *after* CVD simulation. Calibrated from the achieved
    /// minima of the 6 CVD-named themes (`tests::gate_cvd_simulation`'s own
    /// measurement, reproduced here so the floor's provenance is documented
    /// alongside it, the same pattern as `DISTINCT_DELTA_E`), measured under
    /// the corrected `simulate_cvd` coefficients (see that function's doc
    /// comment - a prior mismatched-matrix bug made these numbers wrong and
    /// several of the palettes below were re-tuned in response):
    /// - brdgme light deuteranopia / brdgme light protanopia (shared
    ///   palette): deuteranopia 15.33 (blue/purple), protanopia 15.43
    ///   (red/brown).
    /// - brdgme dark deuteranopia / brdgme dark protanopia (shared palette):
    ///   deuteranopia 12.75 (green/grey), protanopia 13.06 (cyan/pink).
    /// - brdgme light tritanopia: tritanopia 11.06 (brown/grey).
    /// - brdgme dark tritanopia: tritanopia 11.58 (purple/brown).
    ///
    /// The worst of these is 11.06; the floor is set at a round number safely
    /// below that, above the >=9 target from the task brief.
    const CVD_DISTINCT_DELTA_E: f64 = 10.0;

    /// Applies to any theme whose name contains a CVD keyword, so future
    /// themes (e.g. GitHub/Modus variants added later) are covered
    /// automatically without editing this test.
    #[test]
    fn gate_cvd_simulation() {
        for (theme_name, _, palette) in crate::themes() {
            let kind = if theme_name.contains("deuteranopia") {
                Some(Cvd::Deuteranopia)
            } else if theme_name.contains("protanopia") {
                Some(Cvd::Protanopia)
            } else if theme_name.contains("tritanopia") {
                Some(Cvd::Tritanopia)
            } else {
                None
            };
            let Some(kind) = kind else { continue };

            let players = palette.player_colors();
            let mut named: Vec<(&str, Color)> = PLAYER_NAMES
                .iter()
                .zip(players.iter())
                .map(|(&n, &c)| (n, c))
                .collect();
            named.push(("grey", palette.grey));
            named.push(("foreground", palette.foreground));

            let simulated: Vec<(&str, Color)> = named
                .iter()
                .map(|&(n, c)| (n, simulate_cvd(c, kind)))
                .collect();

            let mut min: Option<(f64, &str, &str)> = None;
            for i in 0..simulated.len() {
                for j in (i + 1)..simulated.len() {
                    let d = delta_e(simulated[i].1, simulated[j].1);
                    if min.map(|m| d < m.0).unwrap_or(true) {
                        min = Some((d, simulated[i].0, simulated[j].0));
                    }
                }
            }
            let (min_delta, a, b) = min.expect("player_colors is non-empty");
            assert!(
                min_delta >= CVD_DISTINCT_DELTA_E,
                "[{}] {:?}-simulated {}/{} too close: deltaE {:.2} < {}",
                theme_name,
                kind,
                a,
                b,
                min_delta,
                CVD_DISTINCT_DELTA_E
            );
        }
    }
}
