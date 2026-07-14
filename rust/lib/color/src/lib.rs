use std::fmt;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub use crate::css::{IN_USE_MIXES, IN_USE_SOFTENS, MixExpression, palette_css_vars};
pub use crate::error::ColorError;
pub use crate::palette::{
    DARK, DRACULA, LIGHT, NamedColor, Palette, ThemeCategory, contrast, contrast_ratio, mix,
    soften, themes,
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
        if self.r / 3 + self.g / 3 + self.b / 3 >= 128 {
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
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn from_hex(s: &str) -> Result<Self, ColorError> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$").unwrap();
        }
        if let Some(cap) = RE.captures_iter(&s.to_lowercase()).next() {
            return Ok(Color {
                r: u8::from_str_radix(&cap[1], 16).unwrap(),
                g: u8::from_str_radix(&cap[2], 16).unwrap(),
                b: u8::from_str_radix(&cap[3], 16).unwrap(),
            });
        }
        Err(ColorError::Parse {
            message: format!(r##"expected "#aabbcc", got "{}""##, s),
        })
    }
}

impl FromStr for Color {
    type Err = ColorError;
    fn from_str(s: &str) -> Result<Self, ColorError> {
        if let Some(c) = named(s) {
            return Ok(*c);
        }
        if let Ok(c) = Color::from_hex(s) {
            return Ok(c);
        }
        Err(ColorError::Parse {
            message: format!(
                r##"could not find color "{}"; supply a known color name or hex code "#aabbcc""##,
                s
            ),
        })
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

// Normalises a color name to lowercase alpha-only before matching.
// Accepts any casing and separator style: "BlueGrey", "blue_grey", "blue-grey" all match.
fn named(name: &str) -> Option<&'static Color> {
    let key: String = name
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase();
    match key.as_str() {
        "red" => Some(&LIGHT.red),
        "pink" => Some(&LIGHT.pink),
        "purple" => Some(&LIGHT.purple),
        "deeppurple" => Some(&LIGHT.purple),
        "indigo" => Some(&LIGHT.blue),
        "blue" => Some(&LIGHT.blue),
        "lightblue" => Some(&LIGHT.blue),
        "cyan" => Some(&LIGHT.cyan),
        "teal" => Some(&LIGHT.cyan),
        "green" => Some(&LIGHT.green),
        "lightgreen" => Some(&LIGHT.green),
        "lime" => Some(&LIGHT.green),
        "yellow" => Some(&LIGHT.yellow),
        "amber" => Some(&LIGHT.orange),
        "orange" => Some(&LIGHT.orange),
        "deeporange" => Some(&LIGHT.orange),
        "brown" => Some(&LIGHT.brown),
        "grey" => Some(&LIGHT.grey),
        "bluegrey" => Some(&LIGHT.cyan),
        "magenta" => Some(&LIGHT.purple),
        "white" => Some(&LIGHT.background),
        "black" => Some(&LIGHT.foreground),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_hex_works() {
        assert_eq!(
            Color::from_hex(&LIGHT.red.hex()).expect("error parsing hex"),
            LIGHT.red
        );
    }

    #[test]
    fn color_from_str_named_works() {
        assert_eq!("Green".parse::<Color>().unwrap(), LIGHT.green);
        assert_eq!("BlueGrey".parse::<Color>().unwrap(), LIGHT.cyan);
        assert_eq!("blue_grey".parse::<Color>().unwrap(), LIGHT.cyan);
        assert_eq!("BLUE_GREY".parse::<Color>().unwrap(), LIGHT.cyan);
        assert_eq!("Amber".parse::<Color>().unwrap(), LIGHT.orange);
    }
}
