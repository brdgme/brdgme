use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row};

use crate::{PlayerState, PubState, die_color};

pub fn render_die(d: u8) -> N {
    N::Bold(vec![N::Fg(
        die_color(d).into(),
        vec![N::text(d.to_string())],
    )])
}

pub fn render_dice(dice: &[u8], delim: &str) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, d) in dice.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(delim));
        }
        nodes.push(render_die(*d));
    }
    N::Group(nodes)
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut out: Vec<N> = vec![];

        out.push(N::Table(vec![
            vec![
                (A::Left, vec![N::Bold(vec![N::text("Remaining dice")])]),
                (A::Left, vec![render_dice(&self.remaining_dice, " ")]),
            ],
            vec![
                (A::Left, vec![N::Bold(vec![N::text("Score this turn")])]),
                (A::Left, vec![N::text(self.turn_score.to_string())]),
            ],
        ]));
        out.push(N::text("\n\n"));

        let mut rows: Vec<Row> = vec![vec![
            (A::Left, vec![N::Bold(vec![N::text("Player")])]),
            (A::Left, vec![N::Bold(vec![N::text("Score")])]),
        ]];
        for p in 0..self.players {
            let mut name_nodes: Vec<N> = vec![N::Player(p)];
            if p == self.first_player {
                name_nodes.push(N::text(" (started)"));
            }
            rows.push(vec![
                (A::Left, name_nodes),
                (A::Left, vec![N::text(self.scores[p].to_string())]),
            ]);
        }
        out.push(N::Table(rows));

        out
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        self.public.render()
    }
}
