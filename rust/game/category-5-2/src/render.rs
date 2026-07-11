use brdgme_color as color;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{Card, END_SCORE, PlayerState, PubState, ROW_MAX, ROWS, cards_heads};

pub fn render_card(c: Card) -> N {
    N::Fg(
        c.color().into(),
        vec![N::Bold(vec![N::text(c.0.to_string())])],
    )
}

fn render_board(board: &[Vec<Card>; ROWS]) -> N {
    let mut rows: Vec<Row> = vec![];
    for (i, row_cards) in board.iter().enumerate() {
        let mut row: Row = vec![(A::Left, vec![N::Bold(vec![N::text(format!("#{}", i + 1))])])];
        for j in 0..ROW_MAX {
            let cell = if j < row_cards.len() {
                render_card(row_cards[j])
            } else {
                N::text("  ")
            };
            row.push((A::Left, vec![cell]));
        }
        row.push((
            A::Left,
            vec![N::text(format!("  {} pts", cards_heads(row_cards)))],
        ));
        rows.push(row);
    }
    table_with_gap(&rows, 2)
}

fn render_hand(hand: &[Card]) -> N {
    let mut sorted = hand.to_vec();
    sorted.sort();
    let mut row: Row = vec![(A::Left, vec![N::Bold(vec![N::text("Your hand:")])])];
    for c in &sorted {
        row.push((A::Left, vec![render_card(*c)]));
    }
    table_with_gap(&[row], 2)
}

fn render_legend() -> N {
    let order: [(i32, color::Color); 5] = [
        (1, color::GREY),
        (2, color::CYAN),
        (3, color::YELLOW),
        (5, color::RED),
        (7, color::PURPLE),
    ];
    let mut nodes: Vec<N> = vec![N::Bold(vec![N::text("Legend:")]), N::text(" ")];
    for (i, (heads, col)) in order.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(", "));
        }
        nodes.push(N::Fg(
            col.into(),
            vec![N::Bold(vec![N::text(format!("{} pts", heads))])],
        ));
    }
    N::Group(nodes)
}

fn render_scores(pub_state: &PubState) -> N {
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Players")])]),
        (A::Left, vec![N::Bold(vec![N::text("Taken")])]),
        (A::Left, vec![N::Bold(vec![N::text("Pts")])]),
    ]];
    for p in 0..pub_state.players {
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Center,
                vec![N::text(
                    pub_state
                        .player_cards_counts
                        .get(p)
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                )],
            ),
            (
                A::Center,
                vec![N::Bold(vec![N::text(
                    pub_state
                        .player_points
                        .get(p)
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                )])],
            ),
        ]);
    }
    table_with_gap(&rows, 2)
}

fn render(pub_state: &PubState, hand: Option<&[Card]>) -> Vec<N> {
    let mut out: Vec<N> = vec![];
    out.push(render_board(&pub_state.board));
    if let Some(h) = hand
        && !h.is_empty()
    {
        out.push(N::text("\n\n"));
        out.push(render_hand(h));
    }
    out.push(N::text("\n\n"));
    out.push(render_legend());
    out.push(N::text("\n\n"));
    out.push(render_scores(pub_state));
    let max_points = pub_state.player_points.iter().copied().max().unwrap_or(0);
    out.push(N::text("\n\n"));
    out.push(N::Group(vec![
        N::Bold(vec![N::text(format!("{} points", END_SCORE - max_points))]),
        N::text(" until the end of the game."),
    ]));
    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(&self.hand))
    }
}
