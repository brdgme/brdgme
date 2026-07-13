use brdgme_color::NamedColor;

use crate::ast::{Col, ColTrans, ColType, Node, TNode};
use crate::transform::{align, canvas, indent, table};

/// A colour that hasn't been resolved to a concrete `Color` yet - it carries
/// enough information for a web renderer to pick a CSS class instead.
///
/// Decision: bare `Mono`/`Inv` transform chains have no real users in the
/// current game crates (an audit of markup builders found only `Contrast`
/// used in practice), so both are folded into `contrast: true` here rather
/// than modelled separately. If a real bare mono/inv usage turns up later,
/// this type will need a proper transform enum instead of the single bool.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum SemanticColType {
    Named {
        color: NamedColor,
        soften: Option<u8>,
    },
    Player(usize),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SemanticCol {
    pub color: SemanticColType,
    pub contrast: bool,
}

impl Col {
    fn to_semantic(&self) -> SemanticCol {
        let color = match self.color {
            ColType::Player(p) => SemanticColType::Player(p),
            ColType::Named { color, soften } => SemanticColType::Named { color, soften },
        };
        let contrast = self
            .transform
            .iter()
            .any(|t| matches!(t, ColTrans::Contrast | ColTrans::Mono | ColTrans::Inv));
        SemanticCol { color, contrast }
    }
}

/// A player as seen by the semantic transform - only the name is needed,
/// since the colour stays symbolic (`SemanticColType::Player(n)`) until the
/// web layer maps player index to a palette slot / CSS variable.
pub struct SemanticPlayer {
    pub name: String,
}

pub fn transform_semantic(input: &[Node], players: &[SemanticPlayer]) -> Vec<TNode<SemanticCol>> {
    let mut ret: Vec<TNode<SemanticCol>> = vec![];
    for n in input {
        match *n {
            Node::Fg(ref c, ref children) => ret.push(TNode::Fg(
                c.to_semantic(),
                transform_semantic(children, players),
            )),
            Node::Bg(ref c, ref children) => ret.push(TNode::Bg(
                c.to_semantic(),
                transform_semantic(children, players),
            )),
            Node::Bold(ref children) => {
                ret.push(TNode::Bold(transform_semantic(children, players)))
            }
            Node::Group(ref children) => ret.extend(transform_semantic(children, players)),
            Node::Text(ref t) => ret.push(TNode::Text(t.to_string())),
            Node::Player(p) => ret.extend(player(p, players)),
            Node::Align(ref a, w, ref c) => {
                ret.extend(align(a, w, &transform_semantic(c, players)))
            }
            Node::Indent(n, ref c) => ret.extend(indent(n, &transform_semantic(c, players))),
            Node::Table(ref rows) => ret.extend(table(rows, |children| {
                transform_semantic(children, players)
            })),
            Node::Canvas(ref els) => {
                ret.extend(canvas(els, |nodes| transform_semantic(nodes, players)))
            }
        }
    }
    ret
}

fn player(p: usize, players: &[SemanticPlayer]) -> Vec<TNode<SemanticCol>> {
    let p_name = players
        .get(p)
        .map(|p| p.name.to_string())
        .unwrap_or_else(|| format!("Player {}", p));
    let p_col = SemanticCol {
        color: SemanticColType::Player(p),
        contrast: false,
    };
    vec![TNode::Bold(vec![TNode::Fg(
        p_col,
        vec![TNode::text(format!("<{}>", p_name))],
    )])]
}

#[cfg(test)]
mod tests {
    use crate::ast::{Align as A, Node as N};

    use super::*;

    #[test]
    fn transform_semantic_named_and_soften_works() {
        let result = transform_semantic(
            &[N::Fg(
                NamedColor::Green.into(),
                vec![N::Bg(
                    Col {
                        color: ColType::Named {
                            color: NamedColor::Foreground,
                            soften: Some(86),
                        },
                        transform: vec![],
                    },
                    vec![N::text("x")],
                )],
            )],
            &[],
        );
        assert_eq!(
            result,
            vec![TNode::Fg(
                SemanticCol {
                    color: SemanticColType::Named {
                        color: NamedColor::Green,
                        soften: None
                    },
                    contrast: false,
                },
                vec![TNode::Bg(
                    SemanticCol {
                        color: SemanticColType::Named {
                            color: NamedColor::Foreground,
                            soften: Some(86),
                        },
                        contrast: false,
                    },
                    vec![TNode::text("x")],
                )],
            )]
        );
    }

    #[test]
    fn transform_semantic_contrast_works() {
        let result = transform_semantic(
            &[N::Fg(
                Col {
                    color: ColType::Named {
                        color: NamedColor::Yellow,
                        soften: None,
                    },
                    transform: vec![ColTrans::Contrast],
                },
                vec![N::text("x")],
            )],
            &[],
        );
        assert_eq!(
            result,
            vec![TNode::Fg(
                SemanticCol {
                    color: SemanticColType::Named {
                        color: NamedColor::Yellow,
                        soften: None,
                    },
                    contrast: true,
                },
                vec![TNode::text("x")],
            )]
        );
    }

    #[test]
    fn transform_semantic_player_out_of_range_works() {
        let result = transform_semantic(&[N::Player(3)], &[]);
        assert_eq!(
            result,
            vec![TNode::Bold(vec![TNode::Fg(
                SemanticCol {
                    color: SemanticColType::Player(3),
                    contrast: false,
                },
                vec![TNode::text("<Player 3>")],
            )])]
        );
    }

    #[test]
    fn transform_semantic_align_works() {
        assert_eq!(
            transform_semantic(&[N::Align(A::Left, 5, vec![N::text("ab")])], &[]),
            vec![TNode::text("ab"), TNode::Text("   ".to_string())]
        );
    }
}
