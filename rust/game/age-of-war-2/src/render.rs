use brdgme_color::{GREEN, GREY};
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, table_with_gap};

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
///
/// Column/spacer parity with Go's `render.Table(cells, rowSpacing,
/// colSpacing)` is not attempted here; this renders every section with the
/// correct wording, ordering and colours, which is what Task 1 requires.
/// Exact spacer-cell layout is finished in the render-parity pass.
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
        let mut row: Vec<N> = vec![N::Fg(GREY.into(), vec![N::text(format!("{}.", i + 1))])];
        let (can_afford, _) = l.can_afford(roll);
        let marker = (state.currently_attacking == Some(idx)
            || state.currently_attacking.is_none())
            && (!state.conquered[idx] || state.castle_owners[idx] != Some(state.current_player))
            && !state.completed_lines.contains(&i)
            && can_afford;
        if marker {
            row.push(N::Bold(vec![N::Fg(GREEN.into(), vec![N::text("X ")])]));
        } else {
            row.push(N::text("  "));
        }
        if state.currently_attacking == Some(idx) && state.completed_lines.contains(&i) {
            row.push(N::Fg(GREY.into(), vec![N::text("complete")]));
        } else {
            row.extend(l.render_row());
        }
        rows.push(vec![(A::Left, row)]);
    }

    table_with_gap(&rows, 1)
}

/// Port of Game.RenderCastles (render.go).
fn render_castles(state: &PubState) -> N {
    let all_castles = castle::castles();
    let mut clan_rows: Vec<Vec<(A, Vec<N>)>> = vec![];
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
        let mut row: Vec<N> = vec![];
        for (i, c) in all_castles.iter().enumerate() {
            if c.clan != clan {
                continue;
            }
            if !row.is_empty() {
                row.push(N::text("      "));
            }
            row.push(render_castle(state, i, &state.current_roll));
        }
        clan_rows.push(vec![(A::Center, row)]);
    }
    table_with_gap(&clan_rows, 1)
}

/// Port of Game.PubRender / Game.PlayerRender (render.go). No hidden
/// information in this game, so both states render identically.
fn render(state: &PubState) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    out.push(N::Bold(vec![N::text("Current roll")]));
    out.push(N::text("\n"));
    let mut roll_line: Vec<N> = vec![];
    for (i, d) in state.current_roll.iter().enumerate() {
        if i > 0 {
            roll_line.push(N::text("   "));
        }
        roll_line.push(d.render());
    }
    out.push(N::Group(roll_line));
    out.push(N::text("\n\n"));

    if let Some(idx) = state.currently_attacking {
        out.push(N::Bold(vec![N::text("Currently attacking")]));
        out.push(N::text("\n\n"));
        out.push(render_castle(state, idx, &state.current_roll));
        out.push(N::text("\n\n"));
    }

    out.push(N::Bold(vec![N::text("Castles")]));
    out.push(N::text("\n\n"));
    out.push(render_castles(state));
    out.push(N::text("\n\n"));

    out.push(N::Bold(vec![N::text("Scores")]));
    out.push(N::text("\n"));
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
    out.push(N::Group(score_line));

    out
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
