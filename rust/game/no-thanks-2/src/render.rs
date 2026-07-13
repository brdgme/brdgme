use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{PlayerState, PubState};

pub fn render_card(card: i32) -> N {
    N::Fg(NamedColor::Blue.into(), vec![N::text(card.to_string())])
}

pub fn render_chips(chips: i32) -> N {
    N::Fg(NamedColor::Green.into(), vec![N::text(chips.to_string())])
}

pub fn render_points(points: i32) -> N {
    N::Fg(NamedColor::Purple.into(), vec![N::text(points.to_string())])
}

fn no_cards() -> N {
    N::Fg(NamedColor::Grey.into(), vec![N::text("no cards")])
}

fn group_sorted(sorted: &[i32]) -> Vec<Vec<i32>> {
    let mut groups: Vec<Vec<i32>> = vec![];
    let mut cur: Vec<i32> = vec![];
    let mut last: Option<i32> = None;
    for &c in sorted {
        if last == Some(c - 1) {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                groups.push(std::mem::take(&mut cur));
            }
            cur.push(c);
        }
        last = Some(c);
    }
    if !cur.is_empty() {
        groups.push(cur);
    }
    groups
}

fn render_cards(hand: &[i32], relevant: Option<i32>) -> N {
    let mut sorted = hand.to_vec();
    sorted.sort();
    let groups = group_sorted(&sorted);
    let mut group_nodes: Vec<N> = vec![];
    for (gi, group) in groups.iter().enumerate() {
        if gi > 0 {
            group_nodes.push(N::text("   "));
        }
        let mut card_nodes: Vec<N> = vec![];
        for (ci, &c) in group.iter().enumerate() {
            if ci > 0 {
                card_nodes.push(N::text(" "));
            }
            let card = render_card(c);
            let card = if relevant.is_some_and(|r| (c - r).abs() == 1) {
                N::Bold(vec![card])
            } else {
                card
            };
            card_nodes.push(card);
        }
        group_nodes.push(N::Group(card_nodes));
    }
    N::Group(group_nodes)
}

fn render(pub_state: &PubState, player: Option<usize>, own_chips: Option<i32>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    if !pub_state.finished {
        out.push(N::Bold(vec![
            N::text("Current card:  "),
            render_card(pub_state.current_card.unwrap()),
        ]));
        out.push(N::text(format!(
            " ({} cards remaining)\n",
            pub_state.remaining_after
        )));
        out.push(N::Bold(vec![
            N::text("Current chips: "),
            render_chips(pub_state.centre_chips),
            N::text("\n\n"),
        ]));

        if let Some(p) = player {
            out.push(N::Bold(vec![N::text("Your hand:  ")]));
            if !pub_state.hands[p].is_empty() {
                out.push(render_cards(&pub_state.hands[p], pub_state.current_card));
            } else {
                out.push(no_cards());
            }
            out.push(N::text("\n"));
            out.push(N::Bold(vec![
                N::text("Your chips: "),
                render_chips(own_chips.unwrap_or_default()),
            ]));
            out.push(N::text("\n\n"));
        }
    }

    let mut header: Row = vec![
        (A::Left, vec![N::Bold(vec![N::text("Players")])]),
        (A::Left, vec![N::Bold(vec![N::text("Cards")])]),
    ];
    if pub_state.finished {
        header.push((A::Left, vec![N::Bold(vec![N::text("Score")])]));
    }
    let mut rows: Vec<Row> = vec![header];
    for p in 0..pub_state.players {
        let mut row: Row = vec![(A::Left, vec![N::Player(p)])];
        if !pub_state.hands[p].is_empty() {
            row.push((
                A::Left,
                vec![render_cards(&pub_state.hands[p], pub_state.current_card)],
            ));
        } else {
            row.push((A::Left, vec![no_cards()]));
        }
        if pub_state.finished {
            row.push((
                A::Left,
                vec![
                    N::Bold(vec![render_chips(pub_state.chips[p])]),
                    N::text(" chips, "),
                    N::Bold(vec![render_points(pub_state.final_scores[p])]),
                    N::text(" points"),
                ],
            ));
        }
        rows.push(row);
    }
    out.push(table_with_gap(&rows, 2));

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(self.chips))
    }
}
