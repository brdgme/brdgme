use brdgme_color as color;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{Colour, Dice, DiceResult, PlayerState, PubState};

/// Port of Go `brdgme.CommaList` over markup nodes: "a", "a and b",
/// "a, b and c".
pub fn comma_list(nodes: Vec<N>) -> N {
    let n = nodes.len();
    if n == 0 {
        return N::text("");
    }
    if n == 1 {
        return nodes.into_iter().next().unwrap();
    }
    let mut out: Vec<N> = vec![];
    for (i, node) in nodes.into_iter().enumerate() {
        if i > 0 {
            if i == n - 1 {
                out.push(N::text(" and "));
            } else {
                out.push(N::text(", "));
            }
        }
        out.push(node);
    }
    N::Group(out)
}

/// Renders a single dice result as the face name coloured by dice colour and
/// bolded. Faithful port of Go `DiceResult.String()` which uses
/// `render.Markup(DiceFaceStrings[face], colour, true)`.
pub fn render_dice_result(dr: DiceResult) -> N {
    N::Fg(
        dr.dice.colour.to_color().into(),
        vec![N::Bold(vec![N::text(dr.face.name())])],
    )
}

/// Renders a comma-list of dice results. Faithful port of Go
/// `DiceResultList.String()`.
pub fn render_dice_result_list(drl: &[DiceResult]) -> N {
    let nodes: Vec<N> = drl.iter().map(|dr| render_dice_result(*dr)).collect();
    comma_list(nodes)
}

/// Renders the cup contents as a comma-list of "N colour" entries, coloured
/// and bolded by colour. Empty cup renders as grey "None". Faithful port of
/// the Go `PubRender` cup section.
fn render_cup(cup: &[Dice]) -> N {
    if cup.is_empty() {
        return N::Fg(color::GREY.into(), vec![N::text("None")]);
    }
    let counts = cup_counts(cup);
    let order = [Colour::Green, Colour::Yellow, Colour::Red];
    let mut parts: Vec<N> = vec![];
    for c in order {
        if let Some(&count) = counts.get(&c)
            && count > 0
        {
            parts.push(N::Fg(
                c.to_color().into(),
                vec![N::Bold(vec![N::text(format!(
                    "{} {}",
                    count,
                    colour_name(c)
                ))])],
            ));
        }
    }
    comma_list(parts)
}

fn cup_counts(cup: &[Dice]) -> std::collections::HashMap<Colour, usize> {
    let mut counts = std::collections::HashMap::new();
    for d in cup {
        *counts.entry(d.colour).or_insert(0) += 1;
    }
    counts
}

fn colour_name(c: Colour) -> &'static str {
    match c {
        Colour::Green => "green",
        Colour::Yellow => "yellow",
        Colour::Red => "red",
    }
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut out: Vec<N> = vec![];

        // Main status table: Brains / Shots / Runners / Kept / In cup.
        out.push(table_with_gap(
            &[
                vec![
                    (A::Right, vec![N::text("Brains")]),
                    (
                        A::Left,
                        vec![N::Bold(vec![N::text(self.round_brains.to_string())])],
                    ),
                ],
                vec![
                    (A::Right, vec![N::text("Shots:")]),
                    (
                        A::Left,
                        vec![N::Bold(vec![N::text(self.round_shotguns.to_string())])],
                    ),
                ],
                vec![
                    (A::Right, vec![N::text("Runners:")]),
                    (A::Left, vec![render_dice_result_list(&self.current_roll)]),
                ],
                vec![
                    (A::Right, vec![N::text("Kept:")]),
                    (A::Left, vec![render_dice_result_list(&self.kept)]),
                ],
                vec![
                    (A::Right, vec![N::text("In cup:")]),
                    (A::Left, vec![render_cup(&self.cup)]),
                ],
            ],
            2,
        ));

        out.push(N::Bold(vec![N::text("\n\n\nScores:\n")]));

        // Scores table.
        let mut rows: Vec<Row> = vec![];
        for p in 0..self.players {
            rows.push(vec![
                (A::Right, vec![N::Player(p)]),
                (
                    A::Left,
                    vec![N::Bold(vec![N::text(self.scores[p].to_string())])],
                ),
            ]);
        }
        out.push(table_with_gap(&rows, 2));

        out
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        self.public.render()
    }
}
