use combine::Parser;

pub use crate::ast::{row_pad, row_pad_cell, Align, Node, Row, TNode};
pub use crate::error::MarkupError;
use crate::parser::markup;
pub use crate::transform::{from_lines, to_lines, transform, Player};

mod ansi;
pub mod ast;
mod error;
mod html;
mod parser;
mod plain;
mod transform;

pub fn html(input: &[TNode]) -> String {
    html::render(input)
}

pub fn ansi(input: &[TNode]) -> String {
    ansi::render(input)
}

pub fn plain(input: &[TNode]) -> String {
    plain::render(input)
}

pub fn from_string(input: &str) -> Result<(Vec<Node>, &str), MarkupError> {
    markup().parse(input).map_err(|_| MarkupError::Parse)
}

pub fn to_string(input: &[Node]) -> String {
    input
        .iter()
        .map(|n| match *n {
            Node::Text(ref t) => t.to_owned(),
            Node::Bold(ref children) => format!("{{{{b}}}}{}{{{{/b}}}}", to_string(children)),
            Node::Fg(ref c, ref children) => format!(
                "{{{{fg {}}}}}{}{{{{/fg}}}}",
                c.markup_args(),
                to_string(children)
            ),
            Node::Bg(ref c, ref children) => format!(
                "{{{{bg {}}}}}{}{{{{/bg}}}}",
                c.markup_args(),
                to_string(children)
            ),
            Node::Player(p) => format!("{{{{player {}}}}}", p),
            Node::Group(ref c) => to_string(c),
            Node::Table(ref rows) => format!(
                "{{{{table}}}}{}{{{{/table}}}}",
                rows.iter()
                    .map(|r| format!(
                        "{{{{row}}}}{}{{{{/row}}}}",
                        r.iter()
                            .map(|(align, children)| format!(
                                "{{{{cell {}}}}}{}{{{{/cell}}}}",
                                align,
                                to_string(children)
                            ))
                            .collect::<Vec<String>>()
                            .join("")
                    ))
                    .collect::<Vec<String>>()
                    .join("")
            ),
            Node::Align(ref al, width, ref children) => format!(
                "{{{{align {} {}}}}}{}{{{{/align}}}}",
                al,
                width,
                to_string(children)
            ),
            Node::Indent(width, ref children) => format!(
                "{{{{indent {}}}}}{}{{{{/indent}}}}",
                width,
                to_string(children)
            ),
            Node::Canvas(ref layers) => format!(
                "{{{{canvas}}}}{}{{{{/canvas}}}}",
                layers
                    .iter()
                    .map(|&(x, y, ref children)| format!(
                        "{{{{layer {} {}}}}}{}{{{{/layer}}}}",
                        x,
                        y,
                        to_string(children)
                    ))
                    .collect::<Vec<String>>()
                    .join("")
            ),
        })
        .collect::<Vec<String>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use brdgme_color::*;

    use crate::ast::{Align as A, Node as N};

    use super::*;

    #[test]
    fn ansi_works() {
        ansi(&transform(
            &[
                N::text("Here is "),
                N::Bold(vec![N::text("something")]),
                N::text(" for "),
                N::Player(0),
                N::text(" and "),
                N::Player(1),
            ],
            &[],
        ));
    }

    #[test]
    fn html_works() {
        html(&transform(
            &[
                N::text("Here is "),
                N::Bold(vec![N::text("something")]),
                N::text(" for "),
                N::Player(0),
                N::text(" and "),
                N::Player(1),
            ],
            &[],
        ));
    }

    #[test]
    fn to_string_works() {
        println!(
            "{}",
            to_string(&[N::Canvas(vec![(
                5,
                10,
                vec![N::Table(vec![vec![(
                    A::Center,
                    vec![N::Fg(
                        AMBER.into(),
                        vec![N::Bg(BLUE.into(), vec![N::Bold(vec![N::text("moo")])],),],
                    ),],
                ),],]),],
            ),])],)
        );
    }
}
