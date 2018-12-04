use crate::ast::{Align, BgRange, Col, ColTrans, ColType, Node, Row, TNode};
use brdgme_color::{player_color, Color};

use std::iter;
use std::cmp;
use std::ops::Range;

pub struct Player {
    pub name: String,
    pub color: Color,
}

impl Col {
    fn to_color(&self, players: &[Player]) -> Color {
        let mut c = match self.color {
            ColType::Player(p) => players
                .get(p)
                .map(|p| p.color)
                .unwrap_or_else(|| player_color(p).to_owned()),
            ColType::RGB(c) => c,
        };
        for tf in &self.transform {
            c = match *tf {
                ColTrans::Mono => c.mono(),
                ColTrans::Inv => c.inv(),
            }
        }
        c
    }
}

pub fn transform(input: &[Node], players: &[Player]) -> Vec<TNode> {
    let mut ret: Vec<TNode> = vec![];
    for n in input {
        match *n {
            // Direct copy nodes.
            Node::Fg(ref c, ref children) => {
                ret.push(TNode::Fg(c.to_color(players), transform(children, players)))
            }
            Node::Bg(ref c, ref children) => {
                ret.push(TNode::Bg(c.to_color(players), transform(children, players)))
            }
            Node::Bold(ref children) => ret.push(TNode::Bold(transform(children, players))),
            Node::Group(ref children) => ret.extend(transform(children, players)),
            Node::Text(ref t) => ret.push(TNode::Text(t.to_string())),
            Node::Player(p) => ret.extend(player(p, players)),
            Node::Align(ref a, w, ref c) => ret.extend(align(a, w, &transform(c, players))),
            Node::Indent(n, ref c) => ret.extend(indent(n, &transform(c, players))),
            Node::Table(ref rows) => ret.extend(table(rows, players)),
            Node::Canvas(ref els) => ret.extend(canvas(els, players)),
        }
    }
    ret
}

fn player(p: usize, players: &[Player]) -> Vec<TNode> {
    let p_name = players
        .get(p)
        .map(|p| p.name.to_string())
        .unwrap_or_else(|| format!("Player {}", p));
    let p_col = players
        .get(p)
        .map(|p| p.color)
        .unwrap_or_else(|| player_color(p).to_owned());
    vec![
        TNode::Bold(vec![
            TNode::Fg(p_col, vec![TNode::text(format!("<{}>", p_name))]),
        ]),
    ]
}

fn table(rows: &[Row], players: &[Player]) -> Vec<TNode> {
    // Transform individual cells and calculate row heights and column widths.
    let mut transformed: Vec<Vec<Vec<Vec<TNode>>>> = vec![];
    let mut widths: Vec<usize> = vec![];
    let mut heights: Vec<usize> = vec![];
    for r in rows {
        let mut row: Vec<Vec<Vec<TNode>>> = vec![];
        let mut row_height: usize = 1;
        for (i, &(_, ref children)) in r.iter().enumerate() {
            let cell_lines = to_lines(&transform(children, players));
            row_height = cmp::max(row_height, cell_lines.len());
            let width = cell_lines
                .iter()
                .fold(0, |width, l| cmp::max(width, TNode::len(l)));
            if i >= widths.len() {
                widths.push(width);
            } else {
                widths[i] = cmp::max(widths[i], width);
            }
            row.push(cell_lines);
        }
        heights.push(row_height);
        transformed.push(row);
    }
    // Second pass, output, padding and aligning where required.
    let mut output: Vec<TNode> = vec![];
    for (ri, r) in rows.iter().enumerate() {
        for line_i in 0..heights[ri] {
            if ri > 0 || line_i > 0 {
                output.push(TNode::text("\n"));
            }
            for (ci, w) in widths.iter().enumerate() {
                if let Some(&(ref al, _)) = r.get(ci) {
                    output.extend(if transformed[ri][ci].len() > line_i {
                        align(al, *w, &transformed[ri][ci][line_i])
                    } else {
                        align(&Align::Left, widths[ci], &[])
                    });
                } else {
                    output.extend(align(&Align::Left, widths[ci], &[]));
                }
            }
        }
    }
    output
}

fn align(a: &Align, width: usize, children: &[TNode]) -> Vec<TNode> {
    let mut aligned: Vec<TNode> = vec![];
    for l in to_lines(children) {
        if !aligned.is_empty() {
            aligned.push(TNode::text("\n"));
        }
        let l_len = TNode::len(&l);
        let diff = cmp::max(width, l_len) - l_len;
        match *a {
            Align::Left => {
                aligned.extend(l);
                if diff > 0 {
                    aligned.push(TNode::Text(iter::repeat(" ").take(diff).collect()));
                }
            }
            Align::Center => {
                let before = diff / 2;
                let after = (diff + 1) / 2;
                if before > 0 {
                    aligned.push(TNode::Text(iter::repeat(" ").take(before).collect()));
                }
                aligned.extend(l);
                if after > 0 {
                    aligned.push(TNode::Text(iter::repeat(" ").take(after).collect()));
                }
            }
            Align::Right => {
                if diff > 0 {
                    aligned.push(TNode::Text(iter::repeat(" ").take(diff).collect()));
                }
                aligned.extend(l);
            }
        }
    }
    aligned
}

fn indent(n: usize, children: &[TNode]) -> Vec<TNode> {
    from_lines(&to_lines(children)
        .iter()
        .map(|l| {
            let mut new_l = vec![TNode::Text(iter::repeat(" ").take(n).collect())];
            new_l.extend(l.clone());
            new_l
        })
        .collect::<Vec<Vec<TNode>>>())
}

/// `to_lines` splits text nodes into multiple text nodes, duplicating parent
/// nodes as necessary.
pub fn to_lines(nodes: &[TNode]) -> Vec<Vec<TNode>> {
    let mut lines: Vec<Vec<TNode>> = vec![];
    let mut line: Vec<TNode> = vec![];
    for n in nodes {
        let n_lines: Vec<Vec<TNode>> = match *n {
            TNode::Fg(ref color, ref children) => to_lines(children)
                .iter()
                .map(|l| vec![TNode::Fg(*color, l.to_owned())])
                .collect(),
            TNode::Bg(ref color, ref children) => to_lines(children)
                .iter()
                .map(|l| vec![TNode::Bg(*color, l.to_owned())])
                .collect(),
            TNode::Bold(ref children) => to_lines(children)
                .iter()
                .map(|l| vec![TNode::Bold(l.to_owned())])
                .collect(),
            TNode::Text(ref text) => text.split('\n').map(|l| vec![TNode::text(l)]).collect(),
        };
        let n_lines_len = n_lines.len();
        if n_lines_len > 0 {
            line.extend(n_lines[0].to_owned());
            if n_lines_len > 1 {
                lines.push(line);
                for l in n_lines.iter().take(n_lines_len - 1).skip(1) {
                    lines.push(l.to_owned());
                }
                line = n_lines[n_lines_len - 1].to_owned();
            }
        }
    }
    lines.push(line);
    lines
}

pub fn from_lines(lines: &[Vec<TNode>]) -> Vec<TNode> {
    lines
        .iter()
        .enumerate()
        .flat_map(|(i, l)| {
            let mut new_l = if i == 0 {
                vec![]
            } else {
                vec![TNode::text("\n")]
            };
            new_l.extend(l.clone());
            new_l
        })
        .collect()
}

fn slice(nodes: &[TNode], range: &Range<usize>) -> Vec<TNode> {
    if range.start >= range.end {
        return vec![];
    }
    let mut s = vec![];
    let mut start = range.start;
    let mut end = range.end;
    for n in nodes {
        let n_len = TNode::len(&[n.clone()]);
        if n_len < start {
            start -= n_len;
            end -= n_len;
            continue;
        }
        let n_s: TNode = match *n {
            TNode::Fg(ref color, ref children) => TNode::Fg(*color, slice(children, &(start..end))),
            TNode::Bg(ref color, ref children) => TNode::Bg(*color, slice(children, &(start..end))),
            TNode::Bold(ref children) => TNode::Bold(slice(children, &(start..end))),
            TNode::Text(ref text) => {
                TNode::Text(text[start..cmp::min(text.len(), end)].to_string())
            }
        };

        let n_s_len = TNode::len(&[n_s.clone()]);
        s.push(n_s);
        end -= cmp::min(start + n_s_len, end);
        if end == 0 {
            break;
        }
        start = 0;
    }
    s
}

fn canvas_line_bg_ranges(cl: &[(usize, Vec<TNode>)]) -> Vec<BgRange> {
    cl.iter()
        .flat_map(|&(offset, ref els)| {
            TNode::bg_ranges(els)
                .iter()
                .map(|bgr| bgr.offset(offset))
                .collect::<Vec<BgRange>>()
        })
        .collect()
}

fn bg_ranges_slice(bgrs: &[BgRange], range: &Range<usize>) -> Vec<BgRange> {
    bgrs.iter()
        .filter_map(|bgr| if bgr.start >= range.end || bgr.end <= range.start {
            None
        } else {
            Some(BgRange {
                start: cmp::max(bgr.start, range.start),
                end: cmp::min(bgr.end, range.end),
                ..*bgr
            })
        })
        .collect()
}

fn canvas(els: &[(usize, usize, Vec<Node>)], players: &[Player]) -> Vec<TNode> {
    // Output is split into lines each with a start position.
    let mut lines: Vec<Vec<(usize, Vec<TNode>)>> = vec![];
    for &(x, y, ref nodes) in els {
        let lines_len = lines.len();
        let node_lines = to_lines(&transform(nodes, players));
        let node_lines_len = node_lines.len();
        if y + node_lines_len > lines_len {
            lines.extend(iter::repeat(vec![]).take(y + node_lines_len - lines_len));
        }
        for (n_i, orig_n_line) in node_lines.iter().enumerate() {
            let n_line_y = y + n_i;
            let n_line_len = TNode::len(orig_n_line);
            // Inherit background colors from existing lines if required.
            let ex_n_line_bgrs = canvas_line_bg_ranges(&lines[n_line_y]);
            let n_line: Vec<TNode> = TNode::bg_ranges(orig_n_line)
                .iter()
                .flat_map(|bgr| match bgr.color {
                    Some(_) => slice(orig_n_line, &(bgr.start..bgr.end)),
                    None => bg_ranges_slice(&ex_n_line_bgrs, &(bgr.start + x..bgr.end + x))
                        .iter()
                        .flat_map(|ex_n_line_bgr| {
                            let n_slice = slice(
                                orig_n_line,
                                &(ex_n_line_bgr.start - x..ex_n_line_bgr.end - x),
                            );
                            match ex_n_line_bgr.color {
                                Some(c) => vec![TNode::Bg(c, n_slice)],
                                None => n_slice,
                            }
                        })
                        .collect(),
                })
                .collect();
            // Remove parts of existing lines which this new line now covers.
            lines[n_line_y] = lines[n_line_y]
                .iter()
                .flat_map(|&(ex_x, ref ex_n_line)| {
                    let ex_n_line_len = TNode::len(ex_n_line);
                    if ex_x >= x && ex_x + ex_n_line_len <= x + n_line_len {
                        // Full overlap, remove.
                        return vec![];
                    }
                    if ex_x > x + n_line_len || x > ex_x + ex_n_line_len {
                        // No overlap, keep.
                        return vec![(ex_x, ex_n_line.clone())];
                    }
                    let mut new_parts = vec![];
                    if x > ex_x {
                        new_parts.push((ex_x, slice(ex_n_line, &(0..x - ex_x))))
                    }
                    if ex_x + ex_n_line_len > x + n_line_len {
                        new_parts.push((
                            x + n_line_len,
                            slice(
                                ex_n_line,
                                &(ex_n_line_len - ((ex_x + ex_n_line_len) - (x + n_line_len))..
                                    ex_n_line_len),
                            ),
                        ));
                    }
                    new_parts
                })
                .collect();
            lines[n_line_y].push((x, n_line.clone()));
        }
    }
    from_lines(&lines
        .iter()
        .map(|l| {
            let mut sorted_l = l.clone();
            sorted_l.sort_by(|&(ref a, _), &(ref b, _)| a.cmp(b));
            let mut last_x = 0;
            sorted_l
                .iter()
                .flat_map(|&(x, ref nodes)| {
                    let ret_nodes = if x > last_x {
                        indent(x - last_x, nodes)
                    } else {
                        nodes.clone()
                    };
                    last_x = x + TNode::len(nodes);
                    ret_nodes
                })
                .collect()
        })
        .collect::<Vec<Vec<TNode>>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use brdgme_color::*;
    use crate::plain::render;
    use crate::ast::{Align as A, Node as N, TNode as TN};

    #[test]
    fn align_works() {
        assert_eq!(
            transform(&vec![N::Align(A::Left, 10, vec![N::text("abc")])], &[]),
            vec![TN::text("abc"), TN::text("       ")]
        );
        assert_eq!(
            transform(&vec![N::Align(A::Center, 10, vec![N::text("abc")])], &[]),
            vec![TN::text("   "), TN::text("abc"), TN::text("    ")]
        );
        assert_eq!(
            transform(&vec![N::Align(A::Right, 10, vec![N::text("abc")])], &[]),
            vec![TN::text("       "), TN::text("abc")]
        );
    }

    #[test]
    fn table_align_works() {
        assert_eq!(
            "           blah     \nheadersome long text".to_string(),
            render(&transform(
                &vec![
                    N::Table(vec![
                        vec![
                            (A::Left, vec![]),
                            (A::Center, vec![N::Fg(GREY.into(), vec![N::text("blah")])]),
                        ],
                        vec![
                            (A::Right, vec![N::text("header")]),
                            (
                                A::Center,
                                vec![
                                    N::text(
                                        "some long \
                                         text",
                                    ),
                                ],
                            ),
                        ],
                    ]),
                ],
                &[],
            ),)
        );
    }

    #[test]
    fn table_in_table_works() {
        let t = vec![
            N::Table(vec![
                vec![(A::Left, vec![N::text("one")])],
                vec![(A::Left, vec![N::text("two")])],
                vec![(A::Left, vec![N::text("three")])],
            ]),
        ];
        assert_eq!(
            render(&transform(&t, &[])),
            render(&transform(
                &vec![N::Table(vec![vec![(A::Left, t.clone())]])],
                &[],
            ),)
        );
    }

    #[test]
    fn to_lines_works() {
        assert_eq!(
            to_lines(&vec![TN::text("one\ntwo")]),
            vec![vec![TN::text("one")], vec![TN::text("two")]]
        );
    }

    #[test]
    fn slice_works() {
        assert_eq!(
            slice(
                &vec![TN::Fg(RED, vec![TN::Bold(vec![TN::text("blah")])])],
                &(1..3),
            ),
            vec![TN::Fg(RED, vec![TN::Bold(vec![TN::text("la")])])]
        );
        assert_eq!(
            slice(
                &vec![
                    TN::Bold(vec![
                        TN::Fg(RED, vec![TN::text("one"), TN::text("two")]),
                        TN::Bg(BLUE, vec![TN::text("three"), TN::text("four")]),
                        TN::Bg(GREY, vec![TN::text("five"), TN::text("six")]),
                    ]),
                ],
                &(10..16),
            ),
            vec![
                TN::Bold(vec![
                    TN::Bg(BLUE, vec![TN::text("e"), TN::text("four")]),
                    TN::Bg(GREY, vec![TN::text("f")]),
                ]),
            ]
        );
    }
}
