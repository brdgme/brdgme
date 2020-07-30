use std::str::FromStr;

use combine::parser::char::{digit, letter, string};
use combine::{attempt, choice, none_of, parser, ParseError, Stream};
use combine::{many, many1, Parser};

use brdgme_color::*;

use crate::ast::{Align, Cell, Col, ColTrans, ColType, Node, Row};

fn markup_<Input>() -> impl Parser<Input, Output = Vec<Node>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many(choice((
        bold(),
        fg(),
        bg(),
        c(),
        player(),
        canvas(),
        table(),
        text(),
        align(),
        indent(),
    )))
}

parser! {
    pub fn markup[Input]()(Input) -> Vec<Node>
    where [Input: Stream<Token = char>]
    {
        markup_()
    }
}

fn bold<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (attempt(string("{{b}}")), markup(), string("{{/b}}"))
        .map(|(_, children, _)| Node::Bold(children))
}

fn parse_u8<Input>() -> impl Parser<Input, Output = u8>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(digit()).map(|s: String| s.parse::<u8>().unwrap())
}

fn parse_usize<Input>() -> impl Parser<Input, Output = usize>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(digit()).map(|s: String| s.parse::<usize>().unwrap())
}

fn fg<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{fg ")),
        col_args(),
        string("}}"),
        markup(),
        string("{{/fg}}"),
    )
        .map(|(_, c, _, children, _)| Node::Fg(c, children))
}

fn bg<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{bg ")),
        col_args(),
        string("}}"),
        markup(),
        string("{{/bg}}"),
    )
        .map(|(_, c, _, children, _)| Node::Bg(c, children))
}

fn col_args<Input>() -> impl Parser<Input, Output = Col>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        choice((col_type_player(), col_type_rgb())),
        many(col_trans()),
    )
        .map(|(ct, trans)| Col {
            color: ct,
            transform: trans,
        })
}

fn col_type_player<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (attempt(string("player(")), parse_usize(), string(")")).map(|(_, p, _)| ColType::Player(p))
}

fn col_type_rgb<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("rgb(")),
        parse_u8(),
        string(","),
        parse_u8(),
        string(","),
        parse_u8(),
        string(")"),
    )
        .map(|(_, r, _, g, _, b, _)| ColType::RGB(Color { r, g, b }))
}

fn col_trans<Input>() -> impl Parser<Input, Output = ColTrans>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string(" | ")),
        choice([string("mono"), string("inv")]),
    )
        .map(|(_, t)| match t {
            "mono" => ColTrans::Mono,
            "inv" => ColTrans::Inv,
            _ => panic!("invalid transform"),
        })
}

/// Backwards compatibility with Go brdgme. Magenta is handled manually as it doesn't exist in this
/// version of brdgme.
fn c<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{c ")),
        many1::<String, _, _>(letter()),
        string("}}"),
        markup(),
        string("{{/c}}"),
    )
        .map(|(_, col, _, children, _)| {
            Node::Fg(
                match col.as_ref() {
                    "magenta" => Some(&PURPLE),
                    _ => named(&col),
                }
                .unwrap_or(&BLACK)
                .to_owned()
                .into(),
                children,
            )
        })
}

fn player<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (attempt(string("{{player ")), parse_usize(), string("}}")).map(|(_, p, _)| Node::Player(p))
}

fn canvas<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{canvas}}")),
        many(layer()),
        string("{{/canvas}}"),
    )
        .map(|(_, layers, _)| Node::Canvas(layers))
}

fn layer<Input>() -> impl Parser<Input, Output = (usize, usize, Vec<Node>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{layer ")),
        parse_usize(),
        string(" "),
        parse_usize(),
        string("}}"),
        markup(),
        string("{{/layer}}"),
    )
        .map(|(_, x, _, y, _, children, _)| (x, y, children))
}

fn table<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{table}}")),
        many(row()),
        string("{{/table}}"),
    )
        .map(|(_, rows, _)| Node::Table(rows))
}

fn row<Input>() -> impl Parser<Input, Output = Row>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (attempt(string("{{row}}")), many(cell()), string("{{/row}}")).map(|(_, cells, _)| cells)
}

fn cell<Input>() -> impl Parser<Input, Output = Cell>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{cell ")),
        align_arg(),
        string("}}"),
        markup(),
        string("{{/cell}}"),
    )
        .map(|(_, al, _, children, _)| (al, children))
}

fn align<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        (string("{{align ")),
        align_arg(),
        string(" "),
        parse_usize(),
        string("}}"),
        markup(),
        string("{{/align}}"),
    )
        .map(|(_, al, _, width, _, children, _)| Node::Align(al, width, children))
}

fn indent<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("{{indent ")),
        parse_usize(),
        string("}}"),
        markup(),
        string("{{/indent}}"),
    )
        .map(|(_, width, _, children, _)| Node::Indent(width, children))
}

fn align_arg<Input>() -> impl Parser<Input, Output = Align>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    choice([string("left"), string("center"), string("right")]).map(|s| Align::from_str(s).unwrap())
}

fn text<Input>() -> impl Parser<Input, Output = Node>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(none_of("{".chars())).map(Node::Text)
}

#[cfg(test)]
mod tests {
    use crate::ast::{Align as A, Col, ColTrans, ColType, Node as N};

    use super::super::to_string;
    use super::*;

    #[test]
    fn parse_works() {
        let expected: Vec<Node> = vec![N::Canvas(vec![(
            5,
            10,
            vec![N::Table(vec![vec![(
                A::Center,
                vec![N::Fg(
                    RED.into(),
                    vec![N::Bg(
                        Col {
                            color: ColType::Player(2),
                            transform: vec![ColTrans::Inv, ColTrans::Mono],
                        },
                        vec![
                            N::Player(5),
                            N::Align(
                                A::Right,
                                10,
                                vec![N::Indent(10, vec![N::text("this is some text")])],
                            ),
                        ],
                    )],
                )],
            )]])],
        )])];
        assert_eq!(
            Ok((expected.clone(), "")),
            markup().parse(to_string(&expected).as_ref())
        );
    }
}
