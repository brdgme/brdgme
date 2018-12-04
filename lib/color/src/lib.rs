#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod error;

use std::fmt;
use std::str::FromStr;
use regex::Regex;
pub use error::ColorError;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn mono(self) -> Color {
        if self.r / 3 + self.g / 3 + self.b / 3 >= 128 {
            WHITE
        } else {
            BLACK
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
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b,)
    }

    pub fn ansi_fg(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b,)
    }

    pub fn ansi_bg(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b,)
    }

    pub fn from_hex(s: &str) -> Result<Self, ColorError> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$").unwrap();
        }
        if let Some(cap) = RE.captures_iter(&s.to_lowercase()).next() {
            return Ok(Color {
                r: u8::from_str_radix(&cap[1], 16).unwrap(),
                g: u8::from_str_radix(&cap[2], 16).unwrap(),
                b: u8::from_str_radix(&cap[3], 16).unwrap(),
            });
        }
        Err(ColorError::Parse {
            message: format!(
                r##"expected input in the format of "#aabbcc", got "{}" "##,
                s
            ),
        })
    }

    pub fn from_rgb(_s: &str) -> Result<Self, ColorError> {
        unimplemented!()
    }
}

impl FromStr for Color {
    type Err = ColorError;
    fn from_str(s: &str) -> Result<Self, ColorError> {
        if let Some(c) = named(s) {
            return Ok(c.to_owned());
        }
        if let Ok(c) = Color::from_hex(s) {
            return Ok(c);
        }
        if let Ok(c) = Color::from_rgb(s) {
            return Ok(c);
        }
        Err(ColorError::Parse {
            message: format!(
                r##"could not find color "{}", please supply a known color name, a hex code in the format "#aabbcc" or RGB in the format "rgb(0,128,255)""##,
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
            fg: &BLACK,
            bg: &WHITE,
            bold: false,
        }
    }
}

impl<'a> Style<'a> {
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

    pub fn html_style(self) -> String {
        format!(
            "font-weight:{};color:{};background-color:{};",
            if self.bold { "bold" } else { "normal" },
            self.fg.hex(),
            self.bg.hex(),
        )
    }
}

pub static RED: Color = Color {
    r: 211,
    g: 47,
    b: 47,
};
pub static PINK: Color = Color {
    r: 194,
    g: 24,
    b: 91,
};
pub static PURPLE: Color = Color {
    r: 123,
    g: 31,
    b: 162,
};
pub static DEEP_PURPLE: Color = Color {
    r: 81,
    g: 45,
    b: 168,
};
pub static INDIGO: Color = Color {
    r: 48,
    g: 63,
    b: 159,
};
pub static BLUE: Color = Color {
    r: 25,
    g: 118,
    b: 210,
};
pub static LIGHT_BLUE: Color = Color {
    r: 2,
    g: 136,
    b: 209,
};
pub static CYAN: Color = Color {
    r: 0,
    g: 151,
    b: 167,
};
pub static TEAL: Color = Color {
    r: 0,
    g: 121,
    b: 107,
};
pub static GREEN: Color = Color {
    r: 56,
    g: 142,
    b: 60,
};
pub static LIGHT_GREEN: Color = Color {
    r: 104,
    g: 159,
    b: 56,
};
pub static LIME: Color = Color {
    r: 175,
    g: 180,
    b: 43,
};
pub static YELLOW: Color = Color {
    r: 251,
    g: 192,
    b: 45,
};
pub static AMBER: Color = Color {
    r: 255,
    g: 160,
    b: 0,
};
pub static ORANGE: Color = Color {
    r: 245,
    g: 124,
    b: 0,
};
pub static DEEP_ORANGE: Color = Color {
    r: 230,
    g: 74,
    b: 25,
};
pub static BROWN: Color = Color {
    r: 93,
    g: 64,
    b: 55,
};
pub static GREY: Color = Color {
    r: 97,
    g: 97,
    b: 97,
};
pub static BLUE_GREY: Color = Color {
    r: 69,
    g: 90,
    b: 100,
};
pub static WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};
pub static BLACK: Color = Color { r: 0, g: 0, b: 0 };

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

pub fn player_colors() -> Vec<&'static Color> {
    vec![&GREEN, &RED, &BLUE, &AMBER, &PURPLE, &BROWN, &BLUE_GREY]
}

pub fn player_color<'a>(player: usize) -> &'a Color {
    let pc = player_colors();
    pc[player % pc.len()]
}

pub fn named(name: &str) -> Option<&'static Color> {
    match name {
        "red" => Some(&RED),
        "pink" => Some(&PINK),
        "purple" => Some(&PURPLE),
        "deep_purple" => Some(&DEEP_PURPLE),
        "indigo" => Some(&INDIGO),
        "blue" => Some(&BLUE),
        "light_blue" => Some(&LIGHT_BLUE),
        "cyan" => Some(&CYAN),
        "teal" => Some(&TEAL),
        "green" => Some(&GREEN),
        "light_green" => Some(&LIGHT_GREEN),
        "lime" => Some(&LIME),
        "yellow" => Some(&YELLOW),
        "amber" => Some(&AMBER),
        "orange" => Some(&ORANGE),
        "deep_orange" => Some(&DEEP_ORANGE),
        "brown" => Some(&BROWN),
        "grey" => Some(&GREY),
        "blue_grey" => Some(&BLUE_GREY),
        "white" => Some(&WHITE),
        "black" => Some(&BLACK),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_hex_works() {
        assert_eq!(Color::from_hex(&RED.hex()).expect("error parsing hex"), RED);
    }
}
