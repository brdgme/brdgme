use brdgme_color::{BLACK, CYAN, GREY, RED};
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row};

use crate::{PlayerState, PubState};

fn render(pub_state: &PubState, _player: Option<usize>, hand: Option<&[u8]>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    let bid = if pub_state.bid_quantity == 0 {
        N::Fg(GREY.into(), vec![N::text("first bid")])
    } else {
        render_bid(pub_state.bid_quantity, pub_state.bid_value)
    };
    out.push(N::Group(vec![N::text("Current bid: "), bid, N::text("\n")]));

    if let Some(h) = hand
        && !h.is_empty()
    {
        let mut dice_nodes: Vec<N> = vec![];
        for (i, d) in h.iter().enumerate() {
            if i > 0 {
                dice_nodes.push(N::text(" "));
            }
            dice_nodes.push(render_die(*d as i32));
        }
        out.push(N::Group(vec![
            N::text("Your dice: "),
            N::Bold(dice_nodes),
            N::text("\n\n"),
        ]));
    }

    let mut rows: Vec<Row> = vec![];
    rows.push(vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Left, vec![N::Bold(vec![N::text("Remaining dice")])]),
    ]);
    for p in 0..pub_state.players {
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Left,
                vec![N::text(format!(
                    "{}",
                    pub_state.remaining_dice.get(p).copied().unwrap_or(0)
                ))],
            ),
        ]);
    }
    out.push(N::Table(rows));

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(&self.dice))
    }
}

pub fn render_bid(quantity: i32, value: i32) -> N {
    let suffix = if quantity > 1 { "s" } else { "" };
    N::Group(vec![
        N::text(format!("{} ", quantity)),
        render_die(value),
        N::text(suffix.to_string()),
    ])
}

pub fn render_die(value: i32) -> N {
    let color = if value == 1 { CYAN } else { BLACK };
    N::Bold(vec![N::Fg(color.into(), vec![N::text(value.to_string())])])
}

pub fn reveal_table(player_dice: &[Vec<u8>], active: &[usize], bid_value: i32) -> N {
    let mut rows: Vec<Row> = vec![];
    for &p in active {
        let mut dice_nodes: Vec<N> = vec![];
        for (i, d) in player_dice[p].iter().enumerate() {
            if i > 0 {
                dice_nodes.push(N::text(" "));
            }
            if *d as i32 == bid_value || *d == 1 {
                dice_nodes.push(N::Fg(RED.into(), vec![N::text(d.to_string())]));
            } else {
                dice_nodes.push(N::text(d.to_string()));
            }
        }
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (A::Left, vec![N::Bold(dice_nodes)]),
        ]);
    }
    N::Table(rows)
}
