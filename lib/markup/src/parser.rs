use combine::{many, Parser, many1};
use combine::char::{digit, letter, string};
use combine::combinator::{choice, none_of, parser, r#try};
use combine::primitives::{ParseResult, Stream};

use std::str::FromStr;

use brdgme_color::*;

use crate::ast::{Align, Cell, Col, ColTrans, ColType, Node, Row};

pub fn parse<I>(input: I) -> ParseResult<Vec<Node>, I>
where
    I: Stream<Item = char>,
{
    many(choice([
        bold,
        fg,
        bg,
        c,
        player,
        canvas,
        table,
        text,
        align,
        indent,
    ])).parse_stream(input)
}

fn bold<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (r#try(string("{{b}}")), parser(parse), string("{{/b}}"))
        .map(|(_, children, _)| Node::Bold(children))
        .parse_stream(input)
}

fn parse_u8<I>(input: I) -> ParseResult<u8, I>
where
    I: Stream<Item = char>,
{
    many1(digit())
        .and_then(|s: String| s.parse::<u8>())
        .parse_stream(input)
}

fn parse_usize<I>(input: I) -> ParseResult<usize, I>
where
    I: Stream<Item = char>,
{
    many1(digit())
        .and_then(|s: String| s.parse::<usize>())
        .parse_stream(input)
}

fn fg<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{fg ")),
        parser(col_args),
        string("}}"),
        parser(parse),
        string("{{/fg}}"),
    ).map(|(_, c, _, children, _)| Node::Fg(c, children))
        .parse_stream(input)
}

fn bg<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{bg ")),
        parser(col_args),
        string("}}"),
        parser(parse),
        string("{{/bg}}"),
    ).map(|(_, c, _, children, _)| Node::Bg(c, children))
        .parse_stream(input)
}

fn col_args<I>(input: I) -> ParseResult<Col, I>
where
    I: Stream<Item = char>,
{
    (
        choice([col_type_player, col_type_rgb]),
        many(parser(col_trans)),
    ).map(|(ct, trans)| {
            Col {
                color: ct,
                transform: trans,
            }
        })
        .parse_stream(input)
}

fn col_type_player<I>(input: I) -> ParseResult<ColType, I>
where
    I: Stream<Item = char>,
{
    (r#try(string("player(")), parser(parse_usize), string(")"))
        .map(|(_, p, _)| ColType::Player(p))
        .parse_stream(input)
}

fn col_type_rgb<I>(input: I) -> ParseResult<ColType, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("rgb(")),
        parser(parse_u8),
        string(","),
        parser(parse_u8),
        string(","),
        parser(parse_u8),
        string(")"),
    ).map(|(_, r, _, g, _, b, _)| {
            ColType::RGB(Color { r: r, g: g, b: b })
        })
        .parse_stream(input)
}

fn col_trans<I>(input: I) -> ParseResult<ColTrans, I>
where
    I: Stream<Item = char>,
{
    (r#try(string(" | ")), choice([string("mono"), string("inv")]))
        .map(|(_, t)| match t {
            "mono" => ColTrans::Mono,
            "inv" => ColTrans::Inv,
            _ => panic!("invalid transform"),
        })
        .parse_stream(input)
}

/// Backwards compatibility with Go brdgme. Magenta is handled manually as it doesn't exist in this
/// version of brdgme.
fn c<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{c ")),
        many1::<String, _>(letter()),
        string("}}"),
        parser(parse),
        string("{{/c}}"),
    ).map(|(_, col, _, children, _)| {
            Node::Fg(
                match col.as_ref() {
                    "magenta" => Some(&PURPLE),
                    _ => named(&col),
                }.unwrap_or(&BLACK)
                    .to_owned()
                    .into(),
                children,
            )
        })
        .parse_stream(input)
}

fn player<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (r#try(string("{{player ")), parser(parse_usize), string("}}"))
        .map(|(_, p, _)| Node::Player(p))
        .parse_stream(input)
}

fn canvas<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{canvas}}")),
        many(parser(layer)),
        string("{{/canvas}}"),
    ).map(|(_, layers, _)| Node::Canvas(layers))
        .parse_stream(input)
}

fn layer<I>(input: I) -> ParseResult<(usize, usize, Vec<Node>), I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{layer ")),
        parser(parse_usize),
        string(" "),
        parser(parse_usize),
        string("}}"),
        parser(parse),
        string("{{/layer}}"),
    ).map(|(_, x, _, y, _, children, _)| (x, y, children))
        .parse_stream(input)
}

fn table<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{table}}")),
        many(parser(row)),
        string("{{/table}}"),
    ).map(|(_, rows, _)| Node::Table(rows))
        .parse_stream(input)
}

fn row<I>(input: I) -> ParseResult<Row, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{row}}")),
        many(parser(cell)),
        string("{{/row}}"),
    ).map(|(_, cells, _)| cells)
        .parse_stream(input)
}

fn cell<I>(input: I) -> ParseResult<Cell, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{cell ")),
        parser(align_arg),
        string("}}"),
        parser(parse),
        string("{{/cell}}"),
    ).map(|(_, al, _, children, _)| (al, children))
        .parse_stream(input)
}

fn align<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{align ")),
        parser(align_arg),
        string(" "),
        parser(parse_usize),
        string("}}"),
        parser(parse),
        string("{{/align}}"),
    ).map(|(_, al, _, width, _, children, _)| {
            Node::Align(al, width, children)
        })
        .parse_stream(input)
}

fn indent<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    (
        r#try(string("{{indent ")),
        parser(parse_usize),
        string("}}"),
        parser(parse),
        string("{{/indent}}"),
    ).map(|(_, width, _, children, _)| Node::Indent(width, children))
        .parse_stream(input)
}

fn align_arg<I>(input: I) -> ParseResult<Align, I>
where
    I: Stream<Item = char>,
{
    choice([string("left"), string("center"), string("right")])
        .map(|s| Align::from_str(s).unwrap())
        .parse_stream(input)
}

fn text<I>(input: I) -> ParseResult<Node, I>
where
    I: Stream<Item = char>,
{
    many1(none_of("{".chars()))
        .map(Node::Text)
        .parse_stream(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::to_string;
    use combine::parser;

    use crate::ast::{Align as A, Col, ColTrans, ColType, Node as N};

    #[test]
    fn parse_works() {
        let expected: Vec<Node> = vec![
            N::Canvas(vec![
                (
                    5,
                    10,
                    vec![
                        N::Table(vec![
                            vec![
                                (
                                    A::Center,
                                    vec![
                                        N::Fg(
                                            RED.into(),
                                            vec![
                                                N::Bg(
                                                    Col {
                                                        color: ColType::Player(2),
                                                        transform: vec![
                                                            ColTrans::Inv,
                                                            ColTrans::Mono,
                                                        ],
                                                    },
                                                    vec![
                                                        N::Player(5),
                                                        N::Align(
                                                            A::Right,
                                                            10,
                                                            vec![
                                                                N::Indent(
                                                                    10,
                                                                    vec![
                                                                        N::text(
                                                                            "this is some text",
                                                                        ),
                                                                    ],
                                                                ),
                                                            ],
                                                        ),
                                                    ],
                                                ),
                                            ],
                                        ),
                                    ],
                                ),
                            ],
                        ]),
                    ],
                ),
            ]),
        ];
        assert_eq!(
            Ok((expected.clone(), "")),
            parser(parse).parse(to_string(&expected).as_ref())
        );
    }
}
