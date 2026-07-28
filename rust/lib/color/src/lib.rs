use std::fmt;

use serde::{Deserialize, Serialize};

pub use crate::css::{IN_USE_MIXES, IN_USE_SOFTENS, MixExpression, palette_css_vars};
pub use crate::error::ColorError;
pub use crate::palette::{
    DARK, DRACULA, LIGHT, NamedColor, PLAYER_COUNT, Palette, ThemeCategory, contrast,
    contrast_ratio, mix, rgb, soften, themes,
};

mod css;
mod error;
mod palette;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn mono(self) -> Color {
        let avg = (u16::from(self.r) + u16::from(self.g) + u16::from(self.b) + 1) / 3;
        if avg >= 128 {
            Color {
                r: 255,
                g: 255,
                b: 255,
            }
        } else {
            Color { r: 0, g: 0, b: 0 }
        }
    }

    pub fn inv(self) -> Color {
        Color {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
        }
    }

    pub fn hex(self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Copy)]
pub struct Style<'a> {
    pub fg: &'a Color,
    pub bg: &'a Color,
    pub bold: bool,
}

impl<'a> Default for Style<'a> {
    fn default() -> Style<'a> {
        Style {
            fg: &LIGHT.foreground,
            bg: &LIGHT.background,
            bold: false,
        }
    }
}

impl Style<'_> {
    pub fn ansi(self) -> String {
        format!(
            "\x1b[{b};38;2;{fgr};{fgg};{fgb};48;2;{bgr};{bgg};{bgb}m",
            b = if self.bold { 1 } else { 0 },
            fgr = self.fg.r,
            fgg = self.fg.g,
            fgb = self.fg.b,
            bgr = self.bg.r,
            bgg = self.bg.g,
            bgb = self.bg.b,
        )
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_boundary() {
        assert_eq!(
            Color {
                r: 128,
                g: 128,
                b: 128
            }
            .mono(),
            Color {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(
            Color {
                r: 127,
                g: 127,
                b: 127
            }
            .mono(),
            Color { r: 0, g: 0, b: 0 }
        );
        assert_eq!(
            Color { r: 0, g: 0, b: 0 }.mono(),
            Color { r: 0, g: 0, b: 0 }
        );
        assert_eq!(
            Color {
                r: 255,
                g: 255,
                b: 255
            }
            .mono(),
            Color {
                r: 255,
                g: 255,
                b: 255
            }
        );
    }

    #[test]
    fn hex_equals_to_string() {
        let c = Color {
            r: 0x12,
            g: 0xab,
            b: 0x34,
        };
        assert_eq!(c.hex(), c.to_string());
    }
}
