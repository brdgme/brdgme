use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N};

use crate::{Good, PlayerState, PubState};

fn render_good(good: Good) -> N {
    N::Fg(
        good.color().into(),
        vec![N::Bold(vec![N::text(good.name())])],
    )
}

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

fn camel_display(count: u32) -> &'static str {
    if count == 0 { "no" } else { "some" }
}

pub(crate) fn comma_list_nodes(nodes: Vec<N>) -> N {
    let len = nodes.len();
    let mut out: Vec<N> = vec![];
    for (i, node) in nodes.into_iter().enumerate() {
        if i > 0 {
            if i == len - 1 {
                out.push(N::text(" and "));
            } else {
                out.push(N::text(", "));
            }
        }
        out.push(node);
    }
    N::Group(out)
}

fn render_token_table(goods: &std::collections::HashMap<Good, Vec<u32>>) -> Vec<N> {
    let trade = Good::trade_goods();
    let max_height = trade
        .iter()
        .map(|g| goods.get(g).map_or(0, |v| v.len()))
        .max()
        .unwrap_or(0);
    let mut rows: Vec<Vec<N>> = vec![];

    let mut header: Vec<N> = vec![];
    for (i, g) in trade.iter().enumerate() {
        if i == 3 {
            header.push(N::text(""));
        }
        header.push(N::Fg(
            g.color().into(),
            vec![N::Bold(vec![N::text(g.name())])],
        ));
    }
    rows.push(header);

    for row_i in 0..max_height {
        let mut row: Vec<N> = vec![];
        for (i, g) in trade.iter().enumerate() {
            if i == 3 {
                row.push(N::text(""));
            }
            let pile = goods.get(g);
            if let Some(values) = pile {
                if values.len() > row_i {
                    let val = values[values.len() - 1 - row_i];
                    row.push(N::Fg(
                        g.color().into(),
                        vec![N::Bold(vec![N::text(val.to_string())])],
                    ));
                } else {
                    row.push(N::text(""));
                }
            } else {
                row.push(N::text(""));
            }
        }
        rows.push(row);
    }
    vec![N::Table(
        rows.into_iter()
            .map(|row| row.into_iter().map(|n| (A::Center, vec![n])).collect())
            .collect(),
    )]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut out: Vec<N> = vec![];

        let remaining_rounds = 3u8.saturating_sub(self.round_wins[0] + self.round_wins[1]);
        out.push(N::Bold(vec![N::text(format!(
            "There {} {} {} remaining.",
            if remaining_rounds == 1 { "is" } else { "are" },
            remaining_rounds,
            if remaining_rounds == 1 {
                "round"
            } else {
                "rounds"
            },
        ))]));

        let leader_text = if self.round_wins[0] > self.round_wins[1] {
            "Player 0 is in the lead."
        } else if self.round_wins[1] > self.round_wins[0] {
            "Player 1 is in the lead."
        } else {
            "Scores are level."
        };
        out.push(N::text(leader_text));
        out.push(N::text("\n"));

        out.push(N::Bold(vec![N::text("Sale prices")]));
        out.push(N::text("\n"));
        out.extend(render_token_table(&self.goods));

        out.push(N::text("\n"));
        out.push(N::Bold(vec![N::text("Bonuses for selling")]));
        let bonus_str = format!(
            "3: {} left  4: {} left  5 or more: {} left",
            self.bonuses.get(&3).copied().unwrap_or(0),
            self.bonuses.get(&4).copied().unwrap_or(0),
            self.bonuses.get(&5).copied().unwrap_or(0),
        );
        out.push(N::text(bonus_str));
        out.push(N::text("\n\n"));

        out.push(N::Bold(vec![N::text("Market")]));
        out.push(N::text("\n"));
        out.extend(render_goods_list(&self.market));
        out.push(N::text(format!(
            "\n\n{} cards left in the deck",
            self.deck_len
        )));

        out
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        let mut out = self.public.render();

        out.push(N::text("\n\n"));
        out.push(N::Bold(vec![N::text("You have")]));
        out.push(N::text("\n"));
        out.extend(render_goods_list(&self.hand));
        out.push(N::text(format!(
            "\n{} {}",
            self.public.camels[self.player],
            if self.public.camels[self.player] == 1 {
                "camel"
            } else {
                "camels"
            }
        )));
        out.push(N::text(format!(
            "\n{} point tokens",
            self.public.token_counts[self.player]
        )));

        let opponent = (self.player + 1) % 2;
        out.push(N::text("\n\n"));
        out.push(N::Bold(vec![N::text("Your opponent has")]));
        out.push(N::text(format!(
            "\n{} goods",
            self.public.hand_sizes[opponent]
        )));
        out.push(N::text(format!(
            "\n{} camels",
            camel_display(self.public.camels[opponent])
        )));
        out.push(N::text(format!(
            "\n{} point tokens",
            self.public.token_counts[opponent]
        )));

        out
    }
}
