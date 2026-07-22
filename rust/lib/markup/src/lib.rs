use combine::Parser;

pub use crate::ast::{
    Align, Node, Row, TNode, comma_list_and, comma_list_or, row_pad, row_pad_cell, table_with_gap,
    table_with_spacer,
};
pub use crate::error::MarkupError;
pub use crate::html_class::{html_class, markup_class_css};
use crate::parser::markup;
pub use crate::semantic::{SemanticCol, SemanticColType, SemanticPlayer, transform_semantic};
pub use crate::transform::{Player, from_lines, to_lines, transform, transform_with_palette};
pub use crate::wrap::word_wrap;

mod ansi;
pub mod ast;
mod error;
mod html;
mod html_class;
mod parser;
mod plain;
mod semantic;
mod transform;
mod wrap;

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
    fn table_with_gap_works() {
        let rendered = plain(&transform(
            &[table_with_gap(
                &[
                    vec![
                        (A::Left, vec![N::text("Player")]),
                        (A::Left, vec![N::text("Blue")]),
                    ],
                    vec![(A::Left, vec![N::Player(0)]), (A::Left, vec![N::text("2")])],
                ],
                2,
            )],
            &[Player {
                name: "Alice".to_string(),
                color: brdgme_color::LIGHT.blue,
            }],
        ));
        assert_eq!(
            vec!["Player   Blue", "<Alice>  2"],
            rendered
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<&str>>()
        );
    }

    #[test]
    fn table_with_gap_zero_works() {
        let rendered = plain(&transform(
            &[table_with_gap(
                &[
                    vec![(A::Left, vec![N::text("a")]), (A::Left, vec![N::text("b")])],
                    vec![
                        (A::Left, vec![N::text("cc")]),
                        (A::Left, vec![N::text("d")]),
                    ],
                ],
                0,
            )],
            &[],
        ));
        assert_eq!(
            vec!["a b", "ccd"],
            rendered
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<&str>>()
        );
    }

    #[test]
    fn table_with_spacer_works() {
        let rendered = plain(&transform(
            &[table_with_spacer(
                &[
                    vec![
                        (A::Left, vec![N::text("a")]),
                        (A::Left, vec![N::text("bb")]),
                    ],
                    vec![
                        (A::Left, vec![N::text("ccc")]),
                        (A::Left, vec![N::text("d")]),
                    ],
                ],
                &[N::text("|")],
            )],
            &[],
        ));
        assert_eq!(
            vec!["a  |bb", "ccc|d"],
            rendered
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<&str>>()
        );
    }

    #[test]
    fn table_with_spacer_uneven_rows_works() {
        // Rows with fewer cells don't get trailing spacers, matching Go's
        // render.Table behaviour.
        let rendered = plain(&transform(
            &[table_with_spacer(
                &[
                    vec![(A::Left, vec![N::text("a")]), (A::Left, vec![N::text("b")])],
                    vec![(A::Left, vec![N::text("c")])],
                ],
                &[N::text("|")],
            )],
            &[],
        ));
        assert_eq!(
            vec!["a|b", "c"],
            rendered
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<&str>>()
        );
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
                        NamedColor::Orange.into(),
                        vec![N::Bg(
                            NamedColor::Blue.into(),
                            vec![N::Bold(vec![N::text("moo")])],
                        ),],
                    ),],
                ),],]),],
            ),])],)
        );
    }
}
