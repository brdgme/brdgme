use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::{Card, Suit, sort_by_suit};
use crate::{PlayerState, PubState, end_points};

fn render_card(card: Card) -> N {
    N::Fg(
        card.suit.color().into(),
        vec![N::Bold(vec![N::text(format!("{}", card))])],
    )
}

fn render_cards(cards: &[Card]) -> Vec<N> {
    let mut sorted = cards.to_vec();
    sort_by_suit(&mut sorted);
    sorted.reverse();
    let mut nodes: Vec<N> = vec![];
    for (i, c) in sorted.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(" "));
        }
        nodes.push(render_card(*c));
    }
    nodes
}

fn centered_row(nodes: Vec<N>) -> Row {
    vec![(A::Center, nodes)]
}

fn blank_row() -> Row {
    vec![]
}

fn render_hand_table(hand: &[Card]) -> N {
    let mut sorted = hand.to_vec();
    sort_by_suit(&mut sorted);
    sorted.reverse();

    let rows: Vec<Row> = sorted
        .iter()
        .map(|c| {
            vec![
                (A::Left, vec![render_card(*c)]),
                (
                    A::Left,
                    vec![N::Fg(
                        c.suit.color().into(),
                        vec![N::text(c.suit.rule_str().to_string())],
                    )],
                ),
            ]
        })
        .collect();
    table_with_gap(&rows, 2)
}

fn render_player_table(pub_state: &PubState, viewer: usize) -> N {
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Left, vec![N::Bold(vec![N::text("Hand")])]),
        (A::Left, vec![N::Bold(vec![N::text("Pts")])]),
        (A::Left, vec![N::Bold(vec![N::text("Palette")])]),
    ]];

    let pl = pub_state.num_players;
    for i in 0..pl {
        let p = (viewer + i) % pl;
        let pal_nodes = if pub_state.eliminated[p] {
            vec![N::Fg(NamedColor::Grey.into(), vec![N::text("Eliminated")])]
        } else {
            render_cards(&pub_state.palettes[p])
        };

        let pts: u32 = pub_state.scored_cards[p].iter().map(|c| c.points()).sum();

        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Center,
                vec![N::Bold(vec![N::text(format!(
                    "{}",
                    pub_state.hand_sizes[p]
                ))])],
            ),
            (A::Center, vec![N::Bold(vec![N::text(format!("{}", pts))])]),
            (A::Left, pal_nodes),
        ]);
    }
    table_with_gap(&rows, 2)
}

fn render_color_table() -> N {
    let mut suits = Suit::ALL.to_vec();
    suits.reverse();

    let rows: Vec<Row> = suits
        .iter()
        .map(|s| {
            vec![
                (
                    A::Left,
                    vec![N::Fg(
                        s.color().into(),
                        vec![N::Bold(vec![N::text(s.name().to_string())])],
                    )],
                ),
                (
                    A::Left,
                    vec![N::Fg(
                        s.color().into(),
                        vec![N::text(s.rule_str().to_string())],
                    )],
                ),
            ]
        })
        .collect();
    table_with_gap(&rows, 2)
}

fn common_rows(pub_state: &PubState) -> Vec<Row> {
    let rule = pub_state
        .discard_pile
        .last()
        .map(|c| c.suit)
        .unwrap_or(Suit::Red);

    vec![
        centered_row(vec![
            N::text("First to "),
            N::Bold(vec![N::text(format!(
                "{}",
                end_points(pub_state.num_players)
            ))]),
            N::text(" points"),
        ]),
        blank_row(),
        centered_row(vec![N::Bold(vec![N::text("Current rule")])]),
        centered_row(vec![N::Fg(
            rule.color().into(),
            vec![N::Bold(vec![N::text(rule.rule_str().to_string())])],
        )]),
    ]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut rows = common_rows(self);
        rows.push(blank_row());
        rows.push(centered_row(vec![
            N::Bold(vec![N::text("Deck remaining:")]),
            N::text(format!(" {}", self.deck_len)),
        ]));
        rows.push(blank_row());
        rows.push(centered_row(vec![render_player_table(self, 0)]));
        rows.push(blank_row());
        rows.push(blank_row());
        rows.push(centered_row(vec![render_color_table()]));
        vec![N::Table(rows)]
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        let mut rows = common_rows(&self.public);
        rows.push(blank_row());
        rows.push(centered_row(vec![N::Bold(vec![N::text("Your hand")])]));
        rows.push(centered_row(vec![render_hand_table(&self.hand)]));
        rows.push(blank_row());
        rows.push(centered_row(vec![
            N::Bold(vec![N::text("Deck remaining:")]),
            N::text(format!(" {}", self.public.deck_len)),
        ]));
        rows.push(blank_row());
        rows.push(centered_row(vec![render_player_table(
            &self.public,
            self.player,
        )]));
        rows.push(blank_row());
        rows.push(blank_row());
        rows.push(centered_row(vec![render_color_table()]));
        vec![N::Table(rows)]
    }
}
