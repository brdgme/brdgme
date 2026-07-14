use std::str::FromStr;

use combine::error::StreamError;
use combine::parser::char::{digit, letter, string};
use combine::stream::StreamErrorFor;
use combine::{ParseError, Stream, attempt, choice, none_of, parser};
use combine::{Parser, many, many1};

use brdgme_color::NamedColor;

use crate::ast::{Align, Cell, Col, ColTrans, ColType, Node, Row};

fn markup_<Input>() -> impl Parser<Input, Output = Vec<Node>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many(choice((
        attempt(bold()),
        attempt(fg()),
        attempt(bg()),
        attempt(c()),
        attempt(player()),
        attempt(canvas()),
        attempt(table()),
        attempt(text()),
        attempt(align()),
        attempt(indent()),
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

/// Parses a percentage, valid only from 0 through 100 inclusive. Shared by
/// `soften` and `mix` so both forms enforce the same bound.
fn parse_pct<Input>() -> impl Parser<Input, Output = u8>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(digit()).and_then(|s: String| {
        s.parse::<u8>()
            .ok()
            .filter(|pct| *pct <= 100)
            .ok_or_else(|| {
                <StreamErrorFor<Input>>::message_static_message("percentage must be 0 through 100")
            })
    })
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
        choice((
            attempt(col_type_player()),
            attempt(col_type_rgb()),
            attempt(col_type_mix()),
            attempt(col_type_soften()),
            attempt(col_type_named()),
        )),
        many(col_trans()),
    )
        .map(|(ct, trans): (ColType, Vec<ColTrans>)| {
            let has_inv = trans.contains(&ColTrans::Inv);
            let has_mono = trans.contains(&ColTrans::Mono);
            let trans = if has_inv && has_mono {
                vec![ColTrans::Contrast]
            } else {
                trans
            };
            Col {
                color: ct,
                transform: trans,
            }
        })
}

fn col_type_player<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (attempt(string("player(")), parse_usize(), string(")")).map(|(_, p, _)| ColType::Player(p))
}

fn resolve_named(name: &str) -> Option<NamedColor> {
    match name {
        "magenta" => Some(NamedColor::Purple),
        "amber" => Some(NamedColor::Orange),
        "black" => Some(NamedColor::Foreground),
        "white" => Some(NamedColor::Background),
        _ => NamedColor::from_str(name).ok(),
    }
}

fn col_type_named<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1::<String, _, _>(letter()).and_then(|name| {
        resolve_named(&name)
            .map(|color| ColType::Named {
                color,
                soften: None,
            })
            .ok_or_else(|| <StreamErrorFor<Input>>::message_static_message("unknown named colour"))
    })
}

fn col_type_soften<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("soften(")),
        many1::<String, _, _>(letter()),
        string(","),
        combine::optional(string(" ")),
        parse_pct(),
        string(")"),
    )
        .and_then(|(_, name, _, _, pct, _)| {
            resolve_named(&name)
                .map(|color| ColType::Named {
                    color,
                    soften: Some(pct),
                })
                .ok_or_else(|| {
                    <StreamErrorFor<Input>>::message_static_message("unknown named colour")
                })
        })
}

fn col_type_mix<Input>() -> impl Parser<Input, Output = ColType>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string("mix(")),
        many1::<String, _, _>(letter()),
        string(","),
        combine::optional(string(" ")),
        many1::<String, _, _>(letter()),
        string(","),
        combine::optional(string(" ")),
        parse_pct(),
        string(")"),
    )
        .and_then(|(_, source, _, _, target, _, _, pct, _)| {
            match (resolve_named(&source), resolve_named(&target)) {
                (Some(source), Some(target)) => Ok(ColType::Mix {
                    source,
                    target,
                    pct,
                }),
                _ => Err(<StreamErrorFor<Input>>::message_static_message(
                    "unknown named colour",
                )),
            }
        })
}

fn rgb_reverse_map(r: u8, g: u8, b: u8) -> ColType {
    let named = match (r, g, b) {
        (211, 47, 47) => Some((NamedColor::Red, None)),
        (194, 24, 91) => Some((NamedColor::Pink, None)),
        (123, 31, 162) => Some((NamedColor::Purple, None)),
        (81, 45, 168) => Some((NamedColor::Purple, None)),
        (48, 63, 159) => Some((NamedColor::Blue, None)),
        (25, 118, 210) => Some((NamedColor::Blue, None)),
        (2, 136, 209) => Some((NamedColor::Blue, None)),
        (0, 151, 167) => Some((NamedColor::Cyan, None)),
        (0, 121, 107) => Some((NamedColor::Cyan, None)),
        (56, 142, 60) => Some((NamedColor::Green, None)),
        (104, 159, 56) => Some((NamedColor::Green, None)),
        (175, 180, 43) => Some((NamedColor::Green, None)),
        (251, 192, 45) => Some((NamedColor::Yellow, None)),
        (255, 160, 0) => Some((NamedColor::Orange, None)),
        (245, 124, 0) => Some((NamedColor::Orange, None)),
        (230, 74, 25) => Some((NamedColor::Orange, None)),
        (93, 64, 55) => Some((NamedColor::Brown, None)),
        (97, 97, 97) => Some((NamedColor::Grey, None)),
        (69, 90, 100) => Some((NamedColor::Cyan, None)),
        (255, 255, 255) => Some((NamedColor::Background, None)),
        (0, 0, 0) => Some((NamedColor::Foreground, None)),
        (220, 220, 220) => Some((NamedColor::Foreground, Some(86))),
        (190, 190, 190) => Some((NamedColor::Foreground, Some(75))),
        (248, 187, 208) => Some((NamedColor::Pink, Some(75))),
        (200, 200, 200) => Some((NamedColor::Foreground, Some(78))),
        (100, 100, 100) => Some((NamedColor::Grey, None)),
        (80, 80, 80) => Some((NamedColor::Grey, None)),
        _ => None,
    };
    match named {
        Some((color, soften)) => ColType::Named { color, soften },
        None => {
            eprintln!(
                "warning: unknown rgb colour rgb({},{},{}), falling back to foreground",
                r, g, b
            );
            ColType::Named {
                color: NamedColor::Foreground,
                soften: None,
            }
        }
    }
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
        .map(|(_, r, _, g, _, b, _)| rgb_reverse_map(r, g, b))
}

fn col_trans<Input>() -> impl Parser<Input, Output = ColTrans>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        attempt(string(" | ")),
        choice([string("mono"), string("inv"), string("contrast")]),
    )
        .map(|(_, t)| match t {
            "mono" => ColTrans::Mono,
            "inv" => ColTrans::Inv,
            "contrast" => ColTrans::Contrast,
            _ => panic!("invalid transform"),
        })
}

/// Backwards compatibility with Go brdgme's legacy `{{c name}}` colour tag.
fn resolve_legacy_c_named(name: &str) -> NamedColor {
    let normalised: String = name
        .chars()
        .filter(|c| c.is_alphabetic())
        .flat_map(|c| c.to_lowercase())
        .collect();
    resolve_named(&normalised).unwrap_or(match normalised.as_str() {
        "deeppurple" => NamedColor::Purple,
        "indigo" => NamedColor::Blue,
        "lightblue" => NamedColor::Blue,
        "teal" => NamedColor::Cyan,
        "lightgreen" => NamedColor::Green,
        "lime" => NamedColor::Green,
        "deeporange" => NamedColor::Orange,
        "bluegrey" => NamedColor::Cyan,
        _ => NamedColor::Foreground,
    })
}

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
            let color = resolve_legacy_c_named(&col);
            Node::Fg(color.into(), children)
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
    fn markup_works() {
        let expected: Vec<Node> = vec![N::Canvas(vec![(
            5,
            10,
            vec![N::Table(vec![vec![(
                A::Center,
                vec![N::Fg(
                    NamedColor::Red.into(),
                    vec![N::Bg(
                        Col {
                            color: ColType::Player(2),
                            transform: vec![ColTrans::Contrast],
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
        // The old `| inv | mono` syntax is still accepted, but the parser
        // normalises it to the single `contrast` transform, so we build the
        // AST above with `Contrast` directly and just confirm the string
        // form round-trips through `to_string`/`markup()`.
        assert_eq!(
            Ok((expected.clone(), "")),
            markup().parse(to_string(&expected).as_ref())
        );

        // Confirm the legacy composed idiom is still accepted and normalised.
        let legacy = "{{bg player(2) | inv | mono}}x{{/bg}}";
        let (parsed, rest) = markup().parse(legacy).unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            parsed,
            vec![N::Bg(
                Col {
                    color: ColType::Player(2),
                    transform: vec![ColTrans::Contrast],
                },
                vec![N::text("x")],
            )]
        );
    }

    #[test]
    fn named_color_alias_parsing_works() {
        let (parsed, rest) = markup().parse("{{fg magenta}}x{{/fg}}").unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Purple.into(), vec![N::text("x")])]
        );

        let (parsed, _) = markup().parse("{{fg amber}}x{{/fg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Orange.into(), vec![N::text("x")])]
        );

        let (parsed, _) = markup().parse("{{fg black}}x{{/fg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Foreground.into(), vec![N::text("x")])]
        );

        let (parsed, _) = markup().parse("{{fg white}}x{{/fg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Background.into(), vec![N::text("x")])]
        );

        let (parsed, _) = markup().parse("{{bg soften(pink, 75)}}x{{/bg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Bg(
                Col {
                    color: ColType::Named {
                        color: NamedColor::Pink,
                        soften: Some(75),
                    },
                    transform: vec![],
                },
                vec![N::text("x")],
            )]
        );

        // No space after comma also parses.
        let (parsed_nospace, _) = markup().parse("{{bg soften(pink,75)}}x{{/bg}}").unwrap();
        assert_eq!(parsed_nospace, parsed);
    }

    #[test]
    fn rgb_reverse_map_works() {
        let (parsed, _) = markup().parse("{{fg rgb(211,47,47)}}x{{/fg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Red.into(), vec![N::text("x")])]
        );

        let (parsed, _) = markup().parse("{{bg rgb(220,220,220)}}x{{/bg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Bg(
                Col {
                    color: ColType::Named {
                        color: NamedColor::Foreground,
                        soften: Some(86),
                    },
                    transform: vec![],
                },
                vec![N::text("x")],
            )]
        );

        let (parsed, _) = markup().parse("{{fg rgb(1,2,3)}}x{{/fg}}").unwrap();
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Foreground.into(), vec![N::text("x")])]
        );
    }

    #[test]
    fn unknown_named_color_fails_to_parse() {
        let result = markup().parse("{{fg gren}}x{{/fg}}");
        if let Ok((nodes, _)) = result {
            assert!(
                !nodes.iter().any(|n| matches!(n, N::Fg(..))),
                "unknown colour name must not silently produce a Fg node: {nodes:?}"
            );
        }
    }

    #[test]
    fn legacy_c_tag_parses() {
        let (parsed, rest) = markup().parse("{{c magenta}}x{{/c}}").unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Purple.into(), vec![N::text("x")])]
        );

        let (parsed, rest) = markup().parse("{{c bluegrey}}x{{/c}}").unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            parsed,
            vec![N::Fg(NamedColor::Cyan.into(), vec![N::text("x")])]
        );
    }

    #[test]
    fn contrast_trans_parses() {
        let (parsed, _) = markup()
            .parse("{{fg player(0) | contrast}}x{{/fg}}")
            .unwrap();
        assert_eq!(
            parsed,
            vec![N::Fg(
                Col {
                    color: ColType::Player(0),
                    transform: vec![ColTrans::Contrast],
                },
                vec![N::text("x")],
            )]
        );
    }

    #[test]
    fn round_trip_named_colors_works() {
        let nodes: Vec<Node> = vec![
            N::Fg(NamedColor::Green.into(), vec![N::text("a")]),
            N::Bg(
                Col {
                    color: ColType::Named {
                        color: NamedColor::Pink,
                        soften: Some(75),
                    },
                    transform: vec![],
                },
                vec![N::text("b")],
            ),
            N::Fg(
                Col {
                    color: ColType::Player(1),
                    transform: vec![ColTrans::Contrast],
                },
                vec![N::text("c")],
            ),
            N::Bg(
                Col {
                    color: ColType::Mix {
                        source: NamedColor::Red,
                        target: NamedColor::Blue,
                        pct: 50,
                    },
                    transform: vec![],
                },
                vec![N::text("d")],
            ),
        ];
        let s = to_string(&nodes);
        assert!(s.contains("mix(red, blue, 50)"));
        let (parsed, rest) = markup().parse(s.as_str()).unwrap();
        assert_eq!(rest, "");
        assert_eq!(parsed, nodes);
    }

    #[test]
    fn mix_parsing_works() {
        let (parsed, rest) = markup().parse("{{bg mix(red, blue, 50)}}x{{/bg}}").unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            parsed,
            vec![N::Bg(
                Col {
                    color: ColType::Mix {
                        source: NamedColor::Red,
                        target: NamedColor::Blue,
                        pct: 50,
                    },
                    transform: vec![],
                },
                vec![N::text("x")],
            )]
        );

        // No space after comma also parses.
        let (parsed_nospace, _) = markup().parse("{{bg mix(red,blue,50)}}x{{/bg}}").unwrap();
        assert_eq!(parsed_nospace, parsed);

        if let Ok((nodes, _)) = markup().parse("{{bg mix(red,blue,101)}}x{{/bg}}") {
            assert!(
                !nodes.iter().any(|node| matches!(node, N::Bg(..))),
                "out-of-range mix must not produce a background node: {nodes:?}"
            );
        }
    }

    #[test]
    fn soften_out_of_range_pct_fails_to_parse() {
        if let Ok((nodes, _)) = markup().parse("{{bg soften(pink,101)}}x{{/bg}}") {
            assert!(
                !nodes.iter().any(|node| matches!(node, N::Bg(..))),
                "out-of-range soften must not produce a background node: {nodes:?}"
            );
        }
    }
}
