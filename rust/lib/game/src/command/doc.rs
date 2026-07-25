use brdgme_color::NamedColor;
use brdgme_markup::Node;

use crate::command::Spec;

#[derive(Clone, Default)]
pub struct Opts {
    pub name: Option<String>,
}

impl Spec {
    pub fn doc(&self) -> Vec<(Vec<Node>, Option<String>)> {
        self.doc_opts(&Opts::default())
    }

    fn doc_opts(&self, opts: &Opts) -> Vec<(Vec<Node>, Option<String>)> {
        match *self {
            Spec::Int { min, max } => vec![(doc_int(min, max), None)],
            Spec::Token(ref token) => vec![(doc_token(token), None)],
            Spec::Enum { ref values, .. } => vec![(doc_enum(values, opts), None)],
            Spec::OneOf(ref specs) => doc_one_of(specs, opts),
            Spec::Chain(ref specs) => vec![doc_chain(specs, opts)],
            Spec::Many {
                ref spec,
                min,
                max,
                ref delim,
            } => doc_many(spec, min, max, delim, opts)
                .map(|d| vec![d])
                .unwrap_or_default(),
            Spec::Opt(ref spec) => doc_opt(spec, opts).map(|d| vec![d]).unwrap_or_default(),
            Spec::Doc {
                ref name,
                ref desc,
                ref spec,
            } => doc_doc(name, desc, spec)
                .map(|d| vec![d])
                .unwrap_or_default(),
            Spec::Player => vec![(vec![Node::text("player")], None)],
            Spec::Space => vec![(vec![Node::text(" ")], None)],
        }
    }
}

fn doc_int(min: Option<i32>, max: Option<i32>) -> Vec<Node> {
    match (min, max) {
        (None, None) => vec![Node::text("#")],
        (Some(min), Some(max)) if min == max => {
            vec![Node::Bold(vec![Node::text(format!("{}", min))])]
        }
        // `#` marks the open end. Substituting `0` would contradict both the
        // parser (which accepts negatives when `min` is None) and
        // `Int::expected_output` ("number N or lower") - lg F11.
        (None, Some(max)) => vec![Node::text(format!("#-{}", max))],
        (Some(min), Some(max)) => vec![Node::text(format!("{}-{}", min, max))],
        (Some(min), None) => vec![Node::text(format!("{}+", min))],
    }
}

fn doc_token(token: &str) -> Vec<Node> {
    vec![Node::Bold(vec![Node::text(token)])]
}

fn doc_enum(values: &[String], opts: &Opts) -> Vec<Node> {
    if let Some(ref name) = opts.name {
        return vec![Node::text(format!("[{}]", name))];
    }
    vec![Node::text(format!("[{}]", values.join(" | ")))]
}

fn join_docs(docs: &[(Vec<Node>, Option<String>)]) -> Option<(Vec<Node>, Option<String>)> {
    match docs.len() {
        0 => None,
        1 => Some(docs[0].to_owned()),
        _ => {
            let mut desc: Option<String> = None;
            let mut nodes: Vec<Node> = vec![Node::text("[")];
            for (i, (doc, desc_opt)) in docs.iter().enumerate() {
                if i == 0 {
                    desc = desc_opt.to_owned();
                } else {
                    nodes.push(Node::text(" | "));
                }
                nodes.extend(doc.to_owned());
            }
            nodes.push(Node::text("]"));
            Some((nodes, desc))
        }
    }
}

fn doc_one_of(specs: &[Spec], opts: &Opts) -> Vec<(Vec<Node>, Option<String>)> {
    specs
        .iter()
        .filter_map(|s| join_docs(&s.doc_opts(opts)))
        .collect()
}

fn doc_chain(specs: &[Spec], opts: &Opts) -> (Vec<Node>, Option<String>) {
    let mut desc: Option<String> = None;
    (
        specs
            .iter()
            .enumerate()
            .flat_map(|(i, s)| match join_docs(&s.doc_opts(opts)) {
                Some((doc, desc_opt)) => {
                    if i == 0 {
                        desc = desc_opt;
                    }
                    doc
                }
                None => vec![],
            })
            .collect(),
        desc,
    )
}

fn doc_many(
    spec: &Spec,
    min: Option<usize>,
    max: Option<usize>,
    _delim: &Option<Box<Spec>>,
    opts: &Opts,
) -> Option<(Vec<Node>, Option<String>)> {
    join_docs(&spec.doc_opts(opts)).and_then(|(mut doc, desc)| match (min, max) {
        // Some combinations expect nothing.
        (_, Some(0)) => None,
        (Some(min), Some(max)) if min > max => None,
        // Like optional
        (Some(0), Some(1)) | (None, Some(1)) => {
            doc.push(Node::text("?"));
            Some((doc, desc))
        }
        // Exactly 1
        (Some(1), Some(1)) => Some((doc, desc)),
        // 0 or more. The `*` and `+` shorthands only apply when `max` is
        // None - otherwise they would hide a bounded maximum (lg F12).
        (None, None) | (Some(0), None) => {
            doc.push(Node::text("*"));
            Some((doc, desc))
        }
        // 1 or more
        (Some(1), None) => {
            doc.push(Node::text("+"));
            Some((doc, desc))
        }
        // Other "or more" prepended with min
        (Some(min), None) => {
            let mut prepended = vec![Node::text(format!("({}+)", min))];
            prepended.extend(doc);
            Some((prepended, desc))
        }
        // All others displayed as range. `min.unwrap_or(0)` is correct here,
        // unlike in doc_int: a Many with no minimum requires zero items.
        (min, Some(max)) => {
            doc.push(Node::text(format!("({}-{})", min.unwrap_or(0), max)));
            Some((doc, desc))
        }
    })
}

fn doc_opt(spec: &Spec, opts: &Opts) -> Option<(Vec<Node>, Option<String>)> {
    join_docs(&spec.doc_opts(opts)).map(|(mut doc, desc)| {
        doc.push(Node::text("?"));
        (doc, desc)
    })
}

fn doc_doc(name: &str, desc: &Option<String>, spec: &Spec) -> Option<(Vec<Node>, Option<String>)> {
    join_docs(&spec.doc_opts(&Opts {
        name: Some(name.to_owned()),
    }))
    .map(|(doc, child_desc)| (doc, desc.to_owned().or(child_desc)))
}

pub fn render(docs: &[(Vec<Node>, Option<String>)]) -> Vec<Node> {
    let mut output: Vec<Node> = vec![];
    for (i, (doc, desc)) in docs.iter().enumerate() {
        if i > 0 {
            output.push(Node::text("\n"));
        }
        if let Some(ref desc) = *desc {
            output.push(Node::Fg(
                NamedColor::Grey.into(),
                vec![Node::text(desc.to_owned())],
            ));
            output.push(Node::text("\n  "));
        }
        output.extend(doc.to_owned());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_int_open_minimum_is_not_rendered_as_zero() {
        // lg F11: `min.unwrap_or(0)` documented Int { min: None, max: Some(5) }
        // as "0-5" while the parser accepts negatives and
        // `Int::expected_output` says "number 5 or lower".
        assert_eq!(doc_int(None, Some(5)), vec![Node::text("#-5")]);
        // Unchanged shapes.
        assert_eq!(doc_int(None, None), vec![Node::text("#")]);
        assert_eq!(doc_int(Some(1), Some(5)), vec![Node::text("1-5")]);
        assert_eq!(doc_int(Some(2), None), vec![Node::text("2+")]);
        assert_eq!(
            doc_int(Some(3), Some(3)),
            vec![Node::Bold(vec![Node::text("3")])]
        );
    }

    fn thing() -> Node {
        Node::Bold(vec![Node::text("thing")])
    }

    fn many_doc(min: Option<usize>, max: Option<usize>) -> Option<Vec<Node>> {
        doc_many(
            &Spec::Token("thing".into()),
            min,
            max,
            &None,
            &Opts::default(),
        )
        .map(|(doc, _)| doc)
    }

    #[test]
    fn doc_many_keeps_a_bounded_max() {
        // lg F12: the `*` and `+` arms shadowed the range arm, so
        // Many { min: Some(1), max: Some(2) } (sushi-go-2, sushizock-2,
        // roll-through-the-ages-2) documented as "thing+" instead of
        // "thing(1-2)".
        assert_eq!(
            many_doc(Some(1), Some(2)),
            Some(vec![thing(), Node::text("(1-2)")])
        );
        assert_eq!(
            many_doc(Some(0), Some(3)),
            Some(vec![thing(), Node::text("(0-3)")])
        );
        assert_eq!(
            many_doc(None, Some(3)),
            Some(vec![thing(), Node::text("(0-3)")])
        );
        // Unbounded shapes keep their shorthand.
        assert_eq!(many_doc(None, None), Some(vec![thing(), Node::text("*")]));
        assert_eq!(
            many_doc(Some(0), None),
            Some(vec![thing(), Node::text("*")])
        );
        assert_eq!(
            many_doc(Some(1), None),
            Some(vec![thing(), Node::text("+")])
        );
        assert_eq!(
            many_doc(Some(2), None),
            Some(vec![Node::text("(2+)"), thing()])
        );
        // Optional-like and exactly-one shapes are unchanged.
        assert_eq!(
            many_doc(Some(0), Some(1)),
            Some(vec![thing(), Node::text("?")])
        );
        assert_eq!(
            many_doc(None, Some(1)),
            Some(vec![thing(), Node::text("?")])
        );
        assert_eq!(many_doc(Some(1), Some(1)), Some(vec![thing()]));
        // Empty ranges still document as nothing.
        assert_eq!(many_doc(Some(0), Some(0)), None);
        assert_eq!(many_doc(Some(2), Some(1)), None);
    }
}
