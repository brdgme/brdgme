use std::fmt;

use serde::{Deserialize, Serialize};

use brdgme_color::*;
use brdgme_markup::Node as N;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Casino {
    Albion,
    Sphinx,
    Vega,
    Tivoli,
    Pioneer,
}

pub static CASINOS: &[Casino] = &[
    Casino::Albion,
    Casino::Sphinx,
    Casino::Vega,
    Casino::Tivoli,
    Casino::Pioneer,
];

pub static ALBION_COLOR: &Color = &Color {
    r: 128,
    g: 64,
    b: 124,
};
pub static SPHINX_COLOR: &Color = &Color {
    r: 128,
    g: 116,
    b: 64,
};
pub static VEGA_COLOR: &Color = &Color {
    r: 64,
    g: 128,
    b: 92,
};
pub static TIVOLI_COLOR: &Color = &Color {
    r: 128,
    g: 124,
    b: 121,
};
pub static PIONEER_COLOR: &Color = &Color {
    r: 128,
    g: 70,
    b: 64,
};

impl Casino {
    pub fn color(self) -> &'static Color {
        match self {
            Casino::Albion => ALBION_COLOR,
            Casino::Sphinx => SPHINX_COLOR,
            Casino::Vega => VEGA_COLOR,
            Casino::Tivoli => TIVOLI_COLOR,
            Casino::Pioneer => PIONEER_COLOR,
        }
    }

    pub fn render(self) -> N {
        let c = self.color();
        N::Bold(vec![N::Bg(
            c.into(),
            vec![N::Fg(
                c.inv().mono().into(),
                vec![N::text(format!(" {} ", self))],
            )],
        )])
    }
}

impl fmt::Display for Casino {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Casino::Albion => "Albion",
                Casino::Sphinx => "Sphinx",
                Casino::Vega => "Vega",
                Casino::Tivoli => "Tivoli",
                Casino::Pioneer => "Pioneer",
            }
        )
    }
}
