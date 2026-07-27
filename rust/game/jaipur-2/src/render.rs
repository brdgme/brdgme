use std::collections::HashMap;

use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{Good, PlayerState, PubState};

fn render_good(good: Good) -> N {
    N::Fg(
        good.color().into(),
        vec![N::Bold(vec![N::text(good.name())])],
    )
}

/// Renders a list of goods separated by double spaces, matching Go's
/// `strings.Join(RenderGoods(...), "  ")`.
pub(crate) fn render_goods_list(goods: &[Good]) -> Vec<N> {
    let mut sorted = goods.to_vec();
    sorted.sort_by_key(|g| *g as u8);
    let mut nodes: Vec<N> = vec![];
    for (i, g) in sorted.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(render_good(*g));
    }
    nodes
}

/// Like `render_goods_list`, but returns each good as its own node list with
/// no spacer nodes in between, suitable for feeding into
/// `brdgme_markup::comma_list_and`.
pub(crate) fn render_goods_items(goods: &[Good]) -> Vec<Vec<N>> {
    let mut sorted = goods.to_vec();
    sorted.sort_by_key(|g| *g as u8);
    sorted.into_iter().map(|g| vec![render_good(g)]).collect()
}

fn camel_display(count: u32) -> &'static str {
    if count == 0 { "no" } else { "some" }
}

fn pluralize(n: u64, singular: &'static str, plural_form: &'static str) -> &'static str {
    if n == 1 { singular } else { plural_form }
}

fn heading_row(text: &'static str) -> Row {
    vec![(A::Center, vec![N::Bold(vec![N::text(text)])])]
}

fn centered_row(nodes: Vec<N>) -> Row {
    vec![(A::Center, nodes)]
}

fn blank_row() -> Row {
    vec![]
}

/// Sale price table: a "Rare" / "Common" header spanning the rare and common
/// good columns, then good names, then the price piles from the bottom up.
fn render_token_table(goods: &HashMap<Good, Vec<u32>>) -> N {
    let trade = Good::trade_goods();
    let max_height = trade
        .iter()
        .map(|g| goods.get(g).map_or(0, |v| v.len()))
        .max()
        .unwrap_or(0);
    let mut rows: Vec<Row> = vec![];

    rows.push(vec![
        (A::Center, vec![]),
        (
            A::Center,
            vec![N::Fg(
                NamedColor::Grey.into(),
                vec![N::Bold(vec![N::text("Rare")])],
            )],
        ),
        (A::Center, vec![]),
        (A::Center, vec![]),
        (A::Center, vec![]),
        (
            A::Center,
            vec![N::Fg(
                NamedColor::Grey.into(),
                vec![N::Bold(vec![N::text("Common")])],
            )],
        ),
        (A::Center, vec![]),
    ]);

    let mut subheading: Row = vec![];
    for (i, g) in trade.iter().enumerate() {
        if i == 3 {
            subheading.push((A::Center, vec![]));
        }
        subheading.push((
            A::Center,
            vec![N::Fg(
                g.color().into(),
                vec![N::Bold(vec![N::text(g.name())])],
            )],
        ));
    }
    rows.push(subheading);

    for row_i in 0..max_height {
        let mut row: Row = vec![];
        for (i, g) in trade.iter().enumerate() {
            if i == 3 {
                row.push((A::Center, vec![]));
            }
            let cell = match goods.get(g) {
                Some(values) if values.len() > row_i => {
                    let val = values[values.len() - 1 - row_i];
                    vec![N::Fg(
                        g.color().into(),
                        vec![N::Bold(vec![N::text(val.to_string())])],
                    )]
                }
                _ => vec![],
            };
            row.push((A::Center, cell));
        }
        rows.push(row);
    }

    table_with_gap(&rows, 2)
}

/// Bonus token counts, one column per sale size, with a 4-space gap between
/// columns.
fn render_bonus_table(bonuses: &HashMap<usize, usize>) -> N {
    let row: Row = vec![
        (
            A::Left,
            vec![
                N::Bold(vec![N::text("3")]),
                N::text(format!(": {} left", bonuses.get(&3).copied().unwrap_or(0))),
            ],
        ),
        (
            A::Left,
            vec![
                N::Bold(vec![N::text("4")]),
                N::text(format!(": {} left", bonuses.get(&4).copied().unwrap_or(0))),
            ],
        ),
        (
            A::Left,
            vec![
                N::Bold(vec![N::text("5 or more")]),
                N::text(format!(": {} left", bonuses.get(&5).copied().unwrap_or(0))),
            ],
        ),
    ];
    table_with_gap(&[row], 4)
}

fn deck_count_row(pub_state: &PubState) -> Row {
    centered_row(vec![
        N::Bold(vec![N::text(pub_state.deck_len.to_string())]),
        N::text(format!(
            " {} left in the deck",
            pluralize(pub_state.deck_len as u64, "card", "cards"),
        )),
    ])
}

/// Rows shared by every render, from the round/leader summary through to the
/// market. Excludes the deck count line, which is always last.
fn common_rows(pub_state: &PubState) -> Vec<Row> {
    let (w0, w1) = (pub_state.round_wins[0], pub_state.round_wins[1]);
    // No "N rounds remaining": the match ends at the first player to reach 2
    // round wins, and a fully tied round is replayed without incrementing
    // either counter, so no count derived from round_wins is trustworthy.
    let leader_text = if w0 > w1 {
        format!("Player 0 leads {w0} - {w1}.")
    } else if w1 > w0 {
        format!("Player 1 leads {w1} - {w0}.")
    } else {
        format!("Round wins are level at {w0} - {w1}.")
    };

    vec![
        centered_row(vec![N::Bold(vec![N::text(
            "First to 2 round wins takes the game.",
        )])]),
        centered_row(vec![N::text(leader_text)]),
        blank_row(),
        heading_row("Sale prices"),
        centered_row(vec![render_token_table(&pub_state.goods)]),
        heading_row("Bonuses for selling"),
        centered_row(vec![render_bonus_table(&pub_state.bonuses)]),
        blank_row(),
        heading_row("Market"),
        centered_row(render_goods_list(&pub_state.market)),
    ]
}

fn you_have_rows(pub_state: &PubState, hand: &[Good], player: usize) -> Vec<Row> {
    vec![
        blank_row(),
        heading_row("You have"),
        centered_row(render_goods_list(hand)),
        centered_row(vec![N::text(format!(
            "{} {}",
            pub_state.camels[player],
            pluralize(pub_state.camels[player] as u64, "camel", "camels"),
        ))]),
        centered_row(vec![N::text(format!(
            "{} {}",
            pub_state.token_counts[player],
            pluralize(
                pub_state.token_counts[player] as u64,
                "point token",
                "point tokens",
            ),
        ))]),
    ]
}

fn opponent_rows(pub_state: &PubState, opponent: usize) -> Vec<Row> {
    vec![
        blank_row(),
        heading_row("Your opponent has"),
        centered_row(vec![N::text(format!(
            "{} {}",
            pub_state.hand_sizes[opponent],
            pluralize(pub_state.hand_sizes[opponent] as u64, "good", "goods"),
        ))]),
        centered_row(vec![N::text(format!(
            "{} camels",
            camel_display(pub_state.camels[opponent]),
        ))]),
        centered_row(vec![N::text(format!(
            "{} {}",
            pub_state.token_counts[opponent],
            pluralize(
                pub_state.token_counts[opponent] as u64,
                "point token",
                "point tokens",
            ),
        ))]),
    ]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut rows = common_rows(self);
        rows.push(blank_row());
        rows.push(deck_count_row(self));
        vec![N::Table(rows)]
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        let mut rows = common_rows(&self.public);
        rows.extend(you_have_rows(&self.public, &self.hand, self.player));
        let opponent = (self.player + 1) % 2;
        rows.extend(opponent_rows(&self.public, opponent));
        rows.push(blank_row());
        rows.push(deck_count_row(&self.public));
        vec![N::Table(rows)]
    }
}
