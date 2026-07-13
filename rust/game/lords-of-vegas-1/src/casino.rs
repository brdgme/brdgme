use std::fmt;

use serde::{Deserialize, Serialize};

use brdgme_color::NamedColor;
use brdgme_markup::Node as N;
use brdgme_markup::ast::Col;

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

impl Casino {
    pub fn color(self) -> NamedColor {
        match self {
            Casino::Albion => NamedColor::Purple,
            Casino::Sphinx => NamedColor::Orange,
            Casino::Vega => NamedColor::Green,
            Casino::Tivoli => NamedColor::Grey,
            Casino::Pioneer => NamedColor::Brown,
        }
    }

    pub fn render(self) -> N {
        let c: Col = self.color().into();
        N::Bold(vec![N::Bg(
            c.clone(),
            vec![N::Fg(c.contrast(), vec![N::text(format!(" {} ", self))])],
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
