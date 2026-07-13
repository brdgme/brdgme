use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::castle::{self, ALL_CLANS, Clan, Die};
use crate::{PlayerState, PubState};

/// Port of Game.ClanConquered (game.go), operating on the public fields
/// carried by `PubState` instead of `Game`.
fn clan_conquered(state: &PubState, clan: Clan) -> (bool, Option<usize>) {
    let all_castles = castle::castles();
    let mut player: Option<usize> = None;
    for (i, c) in all_castles.iter().enumerate() {
        if c.clan != clan {
            continue;
        }
        if !state.conquered[i] {
            return (false, None);
        }
        match player {
            None => player = state.castle_owners[i],
            Some(p) => {
                if state.castle_owners[i] != Some(p) {
                    return (false, player);
                }
            }
        }
    }
    (true, player)
}

/// Port of Game.RenderCastle (render.go).
pub fn render_castle(state: &PubState, idx: usize, roll: &[Die]) -> N {
    let all_castles = castle::castles();
    let c = &all_castles[idx];
    let mut rows: Vec<Vec<(A, Vec<N>)>> = vec![];

    rows.push(vec![(
        A::Center,
        vec![c.render_name(), N::text(format!(" ({})", c.points))],
    )]);

    if state.conquered[idx] {
        rows.push(vec![(
            A::Center,
            vec![
                N::text("("),
                N::Player(state.castle_owners[idx].expect("conquered castle has an owner")),
                N::text(")"),
            ],
        )]);
    }

    let lines = c.calc_lines(state.conquered[idx]);
    for (i, l) in lines.iter().enumerate() {
        // Each of these becomes its own cell (matching Go's []render.Cell),
        // joined via a nested colSpacing=1 table below.
        let mut cells: Row = vec![(
            A::Left,
            vec![N::Fg(
                NamedColor::Grey.into(),
                vec![N::text(format!("{}.", i + 1))],
            )],
        )];
        let (can_afford, _) = l.can_afford(roll);
        let marker = (state.currently_attacking == Some(idx)
            || state.currently_attacking.is_none())
            && (!state.conquered[idx] || state.castle_owners[idx] != Some(state.current_player))
            && !state.completed_lines.contains(&i)
            && can_afford;
        if marker {
            cells.push((
                A::Left,
                vec![N::Bold(vec![N::Fg(
                    NamedColor::Green.into(),
                    vec![N::text("X ")],
                )])],
            ));
        } else {
            cells.push((A::Left, vec![N::text("  ")]));
        }
        if state.currently_attacking == Some(idx) && state.completed_lines.contains(&i) {
            cells.push((
                A::Left,
                vec![N::Fg(NamedColor::Grey.into(), vec![N::text("complete")])],
            ));
        } else {
            for n in l.render_row() {
                cells.push((A::Left, vec![n]));
            }
        }
        rows.push(vec![(A::Left, vec![table_with_gap(&[cells], 1)])]);
    }

    table_with_gap(&rows, 0)
}

/// Port of Game.RenderCastles (render.go).
fn render_castles(state: &PubState) -> N {
    let all_castles = castle::castles();
    let mut clan_rows: Vec<Row> = vec![];
    for &clan in &ALL_CLANS {
        let (conquered, by) = clan_conquered(state, clan);
        if conquered {
            clan_rows.push(vec![(
                A::Center,
                vec![
                    clan.render(),
                    N::text(" has been conquered by "),
                    N::Player(by.expect("conquered clan has an owner")),
                    N::text(" for "),
                    N::Bold(vec![N::text(clan.set_points().to_string())]),
                    N::text(" points"),
                ],
            )]);
            continue;
        }
        // Each castle in the clan is its own centered cell, joined via a
        // nested colSpacing=6 table (matching Go's row/Table(row, 0, 6)).
        let mut row: Row = vec![];
        for (i, c) in all_castles.iter().enumerate() {
            if c.clan != clan {
                continue;
            }
            row.push((
                A::Center,
                vec![render_castle(state, i, &state.current_roll)],
            ));
        }
        clan_rows.push(vec![(A::Center, vec![table_with_gap(&[row], 6)])]);
    }
    // rowSpacing=1: a blank row is inserted between each clan block.
    let mut spaced_rows: Vec<Row> = vec![];
    for (i, r) in clan_rows.into_iter().enumerate() {
        if i > 0 {
            spaced_rows.push(vec![(A::Left, vec![N::text("")])]);
        }
        spaced_rows.push(r);
    }
    N::Table(spaced_rows)
}

/// Port of Game.PubRender / Game.PlayerRender (render.go). No hidden
/// information in this game, so both states render identically.
///
/// Ports `render.Layout(layout)`: each element of Go's `layout []string`
/// becomes its own centered row in a colSpacing=0, rowSpacing=0 table,
/// rather than being concatenated inline - this centers headers/blocks
/// against the widest row.
fn render(state: &PubState) -> Vec<N> {
    let mut layout: Vec<Row> = vec![];

    layout.push(vec![(
        A::Center,
        vec![N::Bold(vec![N::text("Current roll")])],
    )]);
    let mut roll_line: Vec<N> = vec![];
    for (i, d) in state.current_roll.iter().enumerate() {
        if i > 0 {
            roll_line.push(N::text("   "));
        }
        roll_line.push(d.render());
    }
    layout.push(vec![(A::Center, roll_line)]);
    layout.push(vec![(A::Center, vec![N::text("")])]);

    if let Some(idx) = state.currently_attacking {
        layout.push(vec![(
            A::Center,
            vec![N::Bold(vec![N::text("Currently attacking")])],
        )]);
        layout.push(vec![(A::Center, vec![N::text("")])]);
        layout.push(vec![(
            A::Center,
            vec![render_castle(state, idx, &state.current_roll)],
        )]);
        layout.push(vec![(A::Center, vec![N::text("")])]);
    }

    layout.push(vec![(A::Center, vec![N::text("")])]);
    layout.push(vec![(A::Center, vec![N::Bold(vec![N::text("Castles")])])]);
    layout.push(vec![(A::Center, vec![N::text("")])]);
    layout.push(vec![(A::Center, vec![render_castles(state)])]);

    layout.push(vec![(A::Center, vec![N::text("")])]);
    layout.push(vec![(A::Center, vec![N::Bold(vec![N::text("Scores")])])]);
    let mut score_line: Vec<N> = vec![];
    for p in 0..state.players {
        if p > 0 {
            score_line.push(N::text("   "));
        }
        score_line.push(N::Player(p));
        score_line.push(N::text(": "));
        score_line.push(N::Bold(vec![N::text(
            state.scores.get(p).copied().unwrap_or(0).to_string(),
        )]));
    }
    layout.push(vec![(A::Center, score_line)]);

    vec![N::Table(layout)]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public)
    }
}
