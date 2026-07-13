use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::{Card, deck_count, princess_to_guard};
use crate::{PlayerState, PubState, plural};

/// Column spacing used by the Go `render.Table(cells, 0, 2)` calls in
/// `render.go` - two literal spacer cells between real columns.
const COL_SPACING: usize = 2;

fn render(pub_state: &PubState, player: Option<usize>, hand: Option<&[Card]>) -> Vec<N> {
    let mut rows: Vec<Row> = vec![];

    rows.push(vec![(
        A::Center,
        vec![
            N::text("The leader has "),
            N::Bold(vec![N::text(format!(
                "{} {}",
                pub_state.leader_points,
                plural(pub_state.leader_points, "point")
            ))]),
            N::text(", the game will end at "),
            N::Bold(vec![N::text(format!("{} points", pub_state.end_score))]),
        ],
    )]);
    rows.push(vec![]);

    if let Some(p) = player {
        if pub_state.eliminated.get(p).copied().unwrap_or(false) {
            rows.push(vec![(
                A::Center,
                vec![N::Bold(vec![N::text(
                    "You have been eliminated from this round",
                )])],
            )]);
        } else if let Some(h) = hand {
            rows.push(vec![(
                A::Center,
                vec![N::Bold(vec![N::text(format!(
                    "Your {}",
                    plural(h.len(), "card")
                ))])],
            )]);
            let mut hand_line: Vec<N> = vec![];
            for (i, &c) in h.iter().enumerate() {
                if i > 0 {
                    hand_line.push(N::text("   "));
                }
                hand_line.push(card(c));
            }
            rows.push(vec![(A::Center, hand_line)]);
        }
    }

    rows.push(vec![(
        A::Center,
        vec![table_with_gap(&player_table(pub_state), COL_SPACING)],
    )]);
    rows.push(vec![]);
    rows.push(vec![(
        A::Center,
        vec![N::Bold(vec![N::text(format!(
            "Cards remaining: {}",
            pub_state.deck_remaining
        ))])],
    )]);

    rows.push(vec![(
        A::Center,
        vec![table_with_gap(&help_table(), COL_SPACING)],
    )]);

    vec![N::Table(rows)]
}

fn player_table(pub_state: &PubState) -> Vec<Row> {
    let mut table: Vec<Row> = vec![
        vec![],
        vec![
            (A::Left, vec![N::Bold(vec![N::text("Player")])]),
            (A::Center, vec![N::Bold(vec![N::text("Pts")])]),
            (A::Center, vec![N::Bold(vec![N::text("Status")])]),
            (A::Center, vec![N::Bold(vec![N::text("Discards")])]),
        ],
    ];
    for p in 0..pub_state.players {
        let status = if pub_state.eliminated.get(p).copied().unwrap_or(false) {
            N::Fg(NamedColor::Grey.into(), vec![N::text("eliminated")])
        } else if pub_state.protected.get(p).copied().unwrap_or(false) {
            N::Bold(vec![N::Fg(
                NamedColor::Foreground.into(),
                vec![N::text("protected")],
            )])
        } else {
            N::Bold(vec![N::Fg(
                NamedColor::Green.into(),
                vec![N::text("active")],
            )])
        };
        let mut discards: Vec<N> = vec![];
        for (i, &c) in pub_state
            .discards
            .get(p)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            if i > 0 {
                discards.push(N::text("  "));
            }
            discards.push(card(c));
        }
        table.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Center,
                vec![N::Bold(vec![N::text(format!(
                    "{}",
                    pub_state.player_points.get(p).copied().unwrap_or(0)
                ))])],
            ),
            (A::Center, vec![status]),
            (A::Left, discards),
        ]);
    }
    table
}

fn help_table() -> Vec<Row> {
    let mut table: Vec<Row> = vec![
        vec![],
        vec![],
        vec![
            (A::Left, vec![N::Bold(vec![N::text("Card")])]),
            (A::Left, vec![N::Bold(vec![N::text("#")])]),
            (A::Left, vec![N::Bold(vec![N::text("Description")])]),
        ],
    ];
    for c in princess_to_guard() {
        table.push(vec![
            (A::Left, vec![card(c)]),
            (A::Left, vec![N::text(format!("{}", deck_count(c)))]),
            (
                A::Left,
                vec![N::Fg(NamedColor::Grey.into(), vec![N::text(c.text())])],
            ),
        ]);
    }
    table
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(&self.hand))
    }
}

pub fn card(c: Card) -> N {
    N::Bold(vec![N::Fg(
        c.color().into(),
        vec![N::text(format!("{} ({})", c.name(), c.number()))],
    )])
}

pub fn comma_cards(cards: &[Card]) -> Vec<N> {
    let mut output: Vec<N> = vec![];
    for &c in cards {
        if !output.is_empty() {
            output.push(N::text(", "));
        }
        output.push(card(c));
    }
    output
}
