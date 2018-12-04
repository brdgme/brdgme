use serde_derive::{Serialize, Deserialize};

use brdgme_color::Color;

use std::str::FromStr;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    pub fn to_string(&self) -> String {
        match *self {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        }.to_string()
    }
}

impl FromStr for Align {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(Align::Left),
            "center" => Ok(Align::Center),
            "right" => Ok(Align::Right),
            _ => Err(format!(
                "invalid align {}, must be one of left, center, right",
                s
            )),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ColTrans {
    Mono,
    Inv,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ColType {
    RGB(Color),
    Player(usize),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Col {
    pub color: ColType,
    pub transform: Vec<ColTrans>,
}

impl Col {
    pub fn markup_args(&self) -> String {
        format!(
            "{}{}",
            self.markup_col_type(),
            match self.transform.len() {
                0 => "".to_string(),
                _ => format!(" | {}", self.markup_trans()),
            }
        )
    }

    fn markup_col_type(&self) -> String {
        match self.color {
            ColType::RGB(c) => format!("rgb({},{},{})", c.r, c.g, c.b),
            ColType::Player(p) => format!("player({})", p),
        }
    }

    fn markup_trans(&self) -> String {
        self.transform
            .iter()
            .map(|t| match *t {
                ColTrans::Mono => "mono".to_string(),
                ColTrans::Inv => "inv".to_string(),
            })
            .collect::<Vec<String>>()
            .join(" | ")
    }

    pub fn inv(&self) -> Self {
        let mut new = self.clone();
        new.transform.push(ColTrans::Inv);
        new
    }

    pub fn mono(&self) -> Self {
        let mut new = self.clone();
        new.transform.push(ColTrans::Mono);
        new
    }
}

impl From<usize> for Col {
    fn from(u: usize) -> Col {
        Col {
            color: ColType::Player(u),
            transform: vec![],
        }
    }
}

impl From<Color> for Col {
    fn from(c: Color) -> Col {
        Col {
            color: ColType::RGB(c),
            transform: vec![],
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    Fg(Col, Vec<Node>),
    Bg(Col, Vec<Node>),
    Group(Vec<Node>),
    Bold(Vec<Node>),
    Text(String),
    Player(usize),
    Table(Vec<Row>),
    Align(Align, usize, Vec<Node>),
    Indent(usize, Vec<Node>),
    Canvas(Vec<(usize, usize, Vec<Node>)>),
}

impl Node {
    pub fn text<T>(t: T) -> Node
    where
        T: Into<String>,
    {
        Node::Text(t.into())
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TNode {
    Fg(Color, Vec<TNode>),
    Bg(Color, Vec<TNode>),
    Bold(Vec<TNode>),
    Text(String),
}

impl TNode {
    pub fn text<T>(t: T) -> TNode
    where
        T: Into<String>,
    {
        TNode::Text(t.into())
    }

    pub fn bg_ranges(nodes: &[TNode]) -> Vec<BgRange> {
        let mut rs: Vec<BgRange> = vec![];
        let mut offset = 0;
        for n in nodes {
            match *n {
                TNode::Text(ref t) => {
                    let cnt = t.chars().count();
                    rs.push(BgRange {
                        start: offset,
                        end: offset + cnt,
                        color: None,
                    });
                    offset += cnt;
                }
                TNode::Bg(c, ref children) => {
                    let mut last_end = 0;
                    for bgr in TNode::bg_ranges(children) {
                        rs.push(BgRange {
                            start: bgr.start + offset,
                            end: bgr.end + offset,
                            color: Some(if let Some(ccol) = bgr.color { ccol } else { c }),
                        });
                        last_end = bgr.end;
                    }
                    offset += last_end;
                }
                TNode::Fg(_, ref children) | TNode::Bold(ref children) => {
                    let mut last_end = 0;
                    for bgr in TNode::bg_ranges(children) {
                        rs.push(bgr.offset(offset));
                        last_end = bgr.end;
                    }
                    offset += last_end;
                }
            }
        }
        rs
    }

    /// Calculates the length of the containing text.  Panics if it detects an untransformed node.
    pub fn len(nodes: &[TNode]) -> usize {
        nodes.iter().fold(0, |sum, n| {
            sum + match *n {
                TNode::Text(ref text) => text.chars().count(),
                TNode::Fg(_, ref children) |
                TNode::Bg(_, ref children) |
                TNode::Bold(ref children) => TNode::len(children),
            }
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct BgRange {
    pub start: usize,
    pub end: usize,
    pub color: Option<Color>,
}

impl BgRange {
    pub fn offset(&self, offset: usize) -> BgRange {
        BgRange {
            start: self.start + offset,
            end: self.end + offset,
            ..*self
        }
    }
}

pub type Row = Vec<Cell>;

pub fn row_pad(row: &[Cell], pad: &str) -> Row {
    row_pad_cell(row, &(Align::Left, vec![Node::text(pad)]))
}

pub fn row_pad_cell(row: &[Cell], pad: &Cell) -> Row {
    row.iter()
        .enumerate()
        .flat_map(|(i, c)| {
            let mut cells: Row = vec![];
            if i > 0 {
                cells.push(pad.to_owned());
            }
            cells.push(c.to_owned());
            cells
        })
        .collect()
}

pub type Cell = (Align, Vec<Node>);

#[cfg(test)]
mod tests {
    use super::*;
    use brdgme_color::*;

    #[test]
    fn bg_ranges_works() {
        assert_eq!(
            vec![
                BgRange {
                    start: 0,
                    end: 9,
                    color: None,
                },
                BgRange {
                    start: 9,
                    end: 14,
                    color: Some(RED),
                },
                BgRange {
                    start: 14,
                    end: 17,
                    color: Some(ORANGE),
                },
                BgRange {
                    start: 17,
                    end: 23,
                    color: Some(RED),
                },
                BgRange {
                    start: 23,
                    end: 32,
                    color: None,
                },
            ],
            TNode::bg_ranges(&vec![
                TNode::text("blah blah"),
                TNode::Bg(
                    RED,
                    vec![
                        TNode::Fg(
                            BLUE,
                            vec![
                                TNode::text("lolol"),
                                TNode::Bg(ORANGE, vec![TNode::text("egg")]),
                                TNode::text("bacon!"),
                            ],
                        ),
                    ],
                ),
                TNode::text("harharhar"),
            ])
        );
    }
}
