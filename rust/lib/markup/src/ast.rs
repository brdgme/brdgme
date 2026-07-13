use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use brdgme_color::Color;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Align::Left => "left",
                Align::Center => "center",
                Align::Right => "right",
            }
        )
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
    Contrast,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ColType {
    Player(usize),
    Named {
        color: brdgme_color::NamedColor,
        soften: Option<u8>,
    },
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
            ColType::Player(p) => format!("player({})", p),
            ColType::Named { color, soften } => match soften {
                Some(pct) => format!("soften({}, {})", color, pct),
                None => color.to_string(),
            },
        }
    }

    fn markup_trans(&self) -> String {
        self.transform
            .iter()
            .map(|t| match *t {
                ColTrans::Mono => "mono".to_string(),
                ColTrans::Inv => "inv".to_string(),
                ColTrans::Contrast => "contrast".to_string(),
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

    pub fn contrast(&self) -> Self {
        let mut new = self.clone();
        new.transform.push(ColTrans::Contrast);
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

impl From<brdgme_color::NamedColor> for Col {
    fn from(c: brdgme_color::NamedColor) -> Col {
        Col {
            color: ColType::Named {
                color: c,
                soften: None,
            },
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

/// A transformed node, generic over its colour representation `C`.
///
/// Layout code (to_lines/from_lines/table/align/canvas in transform.rs) only
/// ever measures text length and rearranges nodes - it never inspects the
/// colour value itself - so it works unchanged for any `C`. The default `C =
/// Color` preserves the original concrete-colour pipeline used by
/// html/ansi/plain rendering; a semantic colour type is plugged in for the
/// web semantic-class renderer (see `semantic.rs`).
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TNode<C = Color> {
    Fg(C, Vec<TNode<C>>),
    Bg(C, Vec<TNode<C>>),
    Bold(Vec<TNode<C>>),
    Text(String),
}

impl<C> TNode<C> {
    pub fn text<T>(t: T) -> TNode<C>
    where
        T: Into<String>,
    {
        TNode::Text(t.into())
    }

    /// Calculates the length of the containing text.  Panics if it detects an untransformed node.
    pub fn len(nodes: &[TNode<C>]) -> usize {
        nodes.iter().fold(0, |sum, n| {
            sum + match *n {
                TNode::Text(ref text) => text.chars().count(),
                TNode::Fg(_, ref children)
                | TNode::Bg(_, ref children)
                | TNode::Bold(ref children) => TNode::len(children),
            }
        })
    }
}

impl<C: Copy> TNode<C> {
    pub fn bg_ranges(nodes: &[TNode<C>]) -> Vec<BgRange<C>> {
        let mut rs: Vec<BgRange<C>> = vec![];
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
}

#[derive(PartialEq, Debug)]
pub struct BgRange<C = Color> {
    pub start: usize,
    pub end: usize,
    pub color: Option<C>,
}

impl<C: Copy> BgRange<C> {
    pub fn offset(&self, offset: usize) -> BgRange<C> {
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

/// Builds a table with `spacer` nodes inserted between adjacent cells in each
/// row. Rows with fewer cells don't get trailing spacers.
pub fn table_with_spacer(rows: &[Row], spacer: &[Node]) -> Node {
    let pad: Cell = (Align::Left, spacer.to_vec());
    Node::Table(rows.iter().map(|r| row_pad_cell(r, &pad)).collect())
}

/// Builds a table with a gap of `gap` spaces between adjacent columns.
pub fn table_with_gap(rows: &[Row], gap: usize) -> Node {
    if gap == 0 {
        return Node::Table(rows.to_vec());
    }
    table_with_spacer(rows, &[Node::Text(" ".repeat(gap))])
}

fn comma_list(items: &[Vec<Node>], last_sep: &str) -> Vec<Node> {
    let mut output: Vec<Node> = vec![];
    let len = items.len();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            output.push(Node::text(if i == len - 1 { last_sep } else { ", " }));
        }
        output.extend(item.to_owned());
    }
    output
}

/// Joins node lists into "a, b and c" style output.
pub fn comma_list_and(items: &[Vec<Node>]) -> Vec<Node> {
    comma_list(items, " and ")
}

/// Joins node lists into "a, b or c" style output.
pub fn comma_list_or(items: &[Vec<Node>]) -> Vec<Node> {
    comma_list(items, " or ")
}

#[cfg(test)]
mod tests {
    use brdgme_color::LIGHT;

    use super::*;

    #[test]
    fn comma_list_and_works() {
        assert_eq!(comma_list_and(&[]), vec![]);
        assert_eq!(
            comma_list_and(&[vec![Node::Player(0)]]),
            vec![Node::Player(0)]
        );
        assert_eq!(
            comma_list_and(&[vec![Node::Player(0)], vec![Node::Player(1)]]),
            vec![Node::Player(0), Node::text(" and "), Node::Player(1)]
        );
        assert_eq!(
            comma_list_and(&[
                vec![Node::text("a")],
                vec![Node::Bold(vec![Node::text("b")])],
                vec![Node::Player(2)],
            ]),
            vec![
                Node::text("a"),
                Node::text(", "),
                Node::Bold(vec![Node::text("b")]),
                Node::text(" and "),
                Node::Player(2),
            ]
        );
    }

    #[test]
    fn comma_list_or_works() {
        assert_eq!(
            comma_list_or(&[
                vec![Node::text("a")],
                vec![Node::text("b")],
                vec![Node::text("c")],
            ]),
            vec![
                Node::text("a"),
                Node::text(", "),
                Node::text("b"),
                Node::text(" or "),
                Node::text("c"),
            ]
        );
    }

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
                    color: Some(LIGHT.red),
                },
                BgRange {
                    start: 14,
                    end: 17,
                    color: Some(LIGHT.orange),
                },
                BgRange {
                    start: 17,
                    end: 23,
                    color: Some(LIGHT.red),
                },
                BgRange {
                    start: 23,
                    end: 32,
                    color: None,
                },
            ],
            TNode::bg_ranges(&[
                TNode::text("blah blah"),
                TNode::Bg(
                    LIGHT.red,
                    vec![TNode::Fg(
                        LIGHT.blue,
                        vec![
                            TNode::text("lolol"),
                            TNode::Bg(LIGHT.orange, vec![TNode::text("egg")]),
                            TNode::text("bacon!"),
                        ],
                    ),],
                ),
                TNode::text("harharhar"),
            ])
        );
    }
}
