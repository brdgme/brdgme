use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{Die, PlayerState, PubState, die_color, scores};

pub fn render_die(d: Die) -> N {
    N::Bold(vec![N::Fg(
        die_color(d).into(),
        vec![N::text(d.to_string())],
    )])
}

pub fn render_dice(dice: &[Die], delim: &str) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, d) in dice.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(delim));
        }
        nodes.push(render_die(*d));
    }
    N::Group(nodes)
}

fn combination_name(dice: &[Die]) -> &'static str {
    match dice {
        [1] => "Single 1",
        [5] => "Single 5",
        [1, 1, 1] => "Three 1s",
        [2, 2, 2] => "Three 2s",
        [3, 3, 3] => "Three 3s",
        [4, 4, 4] => "Three 4s",
        [5, 5, 5] => "Three 5s",
        [6, 6, 6] => "Three 6s",
        _ => unreachable!("every combination in scores() has a display name"),
    }
}

fn scoring_table() -> Vec<Row> {
    let mut table: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Combination")])]),
        (A::Right, vec![N::Bold(vec![N::text("Points")])]),
    ]];
    let mut combos = scores().iter().collect::<Vec<_>>();
    combos.sort_by_key(|s| (s.dice.len(), s.dice[0]));
    for s in combos {
        table.push(vec![
            (A::Left, vec![N::text(combination_name(&s.dice))]),
            (A::Right, vec![N::text(s.value.to_string())]),
        ]);
    }
    table
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut out: Vec<N> = vec![];

        out.push(table_with_gap(
            &[
                vec![
                    (A::Left, vec![N::Bold(vec![N::text("Remaining dice")])]),
                    (A::Left, vec![render_dice(&self.remaining_dice, " ")]),
                ],
                vec![
                    (A::Left, vec![N::Bold(vec![N::text("Score this turn")])]),
                    (A::Left, vec![N::text(self.turn_score.to_string())]),
                ],
            ],
            1,
        ));
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
        out.push(table_with_gap(&rows, 1));

        out.push(N::text("\n\n"));
        out.push(table_with_gap(&scoring_table(), 2));

        out
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        self.public.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoring_table_matches_scores() {
        // The rendered table is derived from scores(), so a combination added
        // to SCORES appears here too - never from a hardcoded list.
        let mut combos = scores().iter().collect::<Vec<_>>();
        combos.sort_by_key(|s| (s.dice.len(), s.dice[0]));
        let mut expected: Vec<Row> = vec![vec![
            (A::Left, vec![N::Bold(vec![N::text("Combination")])]),
            (A::Right, vec![N::Bold(vec![N::text("Points")])]),
        ]];
        for s in combos {
            expected.push(vec![
                (A::Left, vec![N::text(combination_name(&s.dice))]),
                (A::Right, vec![N::text(s.value.to_string())]),
            ]);
        }
        assert_eq!(expected, scoring_table());
        assert_eq!(scoring_table().len(), scores().len() + 1);
    }

    #[test]
    #[should_panic]
    fn test_combination_name_rejects_unknown_combination() {
        // A combination without a display name must fail loudly, so a new
        // SCORES entry cannot silently render as "Unknown".
        combination_name(&[2, 3]);
    }
}
